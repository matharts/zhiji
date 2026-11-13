use super::record::{read_exact_at, write_all_at};
use super::*;
use crate::annotation::{Ink, Mark, MarkKind, Weight};
use std::time::{Duration, Instant};

/// 构造可按编号核对的历史记录。
fn entry(index: usize) -> Undo {
    Undo::Changed {
        index,
        before: Mark {
            ink: Ink::Blue,
            weight: Weight::Medium,
            stroke_width: 2.5,
            kind: MarkKind::Pen(
                (0..600)
                    .map(|i| [index as f32 + i as f32 * 0.125, 30.0 + i as f32 * 0.25])
                    .collect(),
            ),
        },
    }
}

/// 在有界等待中轮询历史，直到后台转存完成。
fn settle(history: &mut History) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        history.poll(None);
        assert!(Instant::now() < deadline, "history spill timed out");
        if history.pending.is_none()
            && (history.resident_bytes <= history.limit || history.error.is_some())
        {
            break;
        }
        std::thread::yield_now();
    }
}

/// 旧往返断言等待真实异步结果；非阻塞语义另由受控读取测试验证。
fn pop(history: &mut History) -> Result<Option<Undo>, String> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        match history.pop(None)? {
            Pop::Ready(entry) => return Ok(Some(entry)),
            Pop::Empty => return Ok(None),
            Pop::Pending => {
                assert!(Instant::now() < deadline, "历史读取未完成");
                std::thread::yield_now();
            }
        }
    }
}

#[test]
/// 被阻塞的读取立即返回 Pending，重复请求不前进历史也不创建新线程。
fn disk_read_does_not_block_or_advance_the_stack() {
    let mut history = History::with_limit(0);
    history.push(entry(1), None);
    settle(&mut history);
    let end = history.disk_end;
    let release = history.hold_read_for_test();
    for _ in 0..20 {
        assert!(matches!(history.pop(None).unwrap(), Pop::Pending));
        assert_eq!(history.disk_end, end);
        assert_eq!(history.disk_count, 1);
    }
    release.send(()).unwrap();
    assert_eq!(pop(&mut history).unwrap(), Some(entry(1)));
    assert!(history.is_empty());
}

#[test]
/// 清空历史立即发出取消信号，旧读取不能影响新的空栈。
fn clearing_history_cancels_an_inflight_read() {
    let mut history = History::with_limit(0);
    history.push(entry(1), None);
    settle(&mut history);
    let release = history.hold_read_for_test();
    assert!(matches!(history.pop(None).unwrap(), Pop::Pending));
    let cancel = history.reading.as_ref().unwrap().cancel.clone();
    history.clear();
    assert!(cancel.is_cancelled());
    assert!(history.is_empty());
    release.send(()).unwrap();
}
#[test]
/// 读取期间的新记录优先出栈，旧读取结果不能重复消费或覆盖新栈顶。
fn edits_during_disk_read_invalidate_the_old_result() {
    let mut history = History::with_limit(0);
    history.push(entry(1), None);
    settle(&mut history);
    let release = history.hold_read_for_test();
    assert!(matches!(history.pop(None).unwrap(), Pop::Pending));
    history.limit = usize::MAX;
    history.push(entry(2), None);
    assert_eq!(pop(&mut history).unwrap(), Some(entry(2)));
    release.send(()).unwrap();
    assert_eq!(pop(&mut history).unwrap(), Some(entry(1)));
    assert!(history.is_empty());
}

#[test]
/// 验证转存不丢记录，并按栈顺序逆向读取。
fn spill_preserves_every_entry_and_reverse_order() {
    let mut history = History::with_limit(2_000);
    let expected: Vec<_> = (0..24).map(entry).collect();
    for item in &expected {
        history.push(item.clone(), None);
    }
    settle(&mut history);
    assert!(history.disk_count > 0);
    assert!(history.resident_bytes <= history.limit);
    for item in expected.into_iter().rev() {
        assert_eq!(pop(&mut history).unwrap(), Some(item));
    }
    assert!(history.is_empty());
}

#[test]
/// 验证所有撤销记录变体可通过磁盘转存往返。
fn all_undo_variants_round_trip_through_spill() {
    let mark = |kind| Mark {
        ink: Ink::Blue,
        weight: Weight::Thick,
        stroke_width: 4.0,
        kind,
    };
    let a = [10.25, 25.5];
    let b = [30.75, 45.125];
    let variants = vec![
        Undo::Added(1),
        Undo::Changed {
            index: 1,
            before: mark(MarkKind::Pen(vec![a, b])),
        },
        Undo::PathChanged {
            index: 1,
            xor: vec![0, 127, 128, 255],
            stroke_xor: 0x89abcdef,
        },
        Undo::Removed {
            index: 1,
            mark: mark(MarkKind::Text {
                origin: a,
                value: "紫微 ✨ A".repeat(400),
                size: 18.0,
            }),
        },
        Undo::Cleared(vec![
            mark(MarkKind::Highlighter(vec![a, b])),
            mark(MarkKind::Line(a, b)),
            mark(MarkKind::Arrow(a, b)),
            mark(MarkKind::Rectangle(a, b)),
            mark(MarkKind::Ellipse(a, b)),
        ]),
        Undo::Reordered { from: 2, to: 0 },
    ];
    let mut history = History::with_limit(0);
    for value in &variants {
        history.push(value.clone(), None);
    }
    settle(&mut history);
    assert_eq!(history.disk_count, variants.len());
    for expected in variants.into_iter().rev() {
        assert_eq!(pop(&mut history).unwrap(), Some(expected));
    }
}

#[test]
/// 验证超出默认内存预算的大载荷会被转存。
fn default_budget_spills_large_history_payloads() {
    let mut history = History::default();
    for index in 0..5 {
        history.push(
            Undo::PathChanged {
                index,
                xor: vec![index as u8; 5 * 1024 * 1024],
                stroke_xor: 0,
            },
            None,
        );
    }
    settle(&mut history);
    assert!(history.disk_count > 0);
    assert!(history.resident_bytes <= RESIDENT_LIMIT);
    for index in (0..5).rev() {
        let Undo::PathChanged {
            index: actual, xor, ..
        } = pop(&mut history).unwrap().unwrap()
        else {
            panic!("wrong undo variant")
        };
        assert_eq!(actual, index);
        assert_eq!(xor.len(), 5 * 1024 * 1024);
        assert!(xor.iter().all(|value| *value == index as u8));
    }
    assert!(history.is_empty());
}

#[test]
/// 验证快速压栈弹栈与纯内存参考栈一致。
fn rapid_push_and_pop_matches_an_in_memory_stack() {
    let mut history = History::with_limit(256);
    let mut reference = Vec::new();
    let mut random = 0x4312_9876u32;
    for index in 0..500 {
        random = random.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        if random.is_multiple_of(4) && !reference.is_empty() {
            assert_eq!(pop(&mut history).unwrap(), reference.pop());
        } else {
            let value = entry(index);
            history.push(value.clone(), None);
            reference.push(value);
        }
    }
    settle(&mut history);
    while let Some(expected) = reference.pop() {
        assert_eq!(pop(&mut history).unwrap(), Some(expected));
    }
    assert!(history.is_empty());
}

#[test]
/// 验证转存期间撤销不会使晚到结果重新引入旧记录。
fn undo_during_spill_cannot_reintroduce_an_old_entry() {
    let mut history = History::with_limit(0);
    let gate = Arc::new(std::sync::Barrier::new(2));
    history.write_gate = Some(Arc::clone(&gate));
    let first = entry(1);
    let second = entry(2);
    history.push(first.clone(), None);
    assert_eq!(pop(&mut history).unwrap(), Some(first));
    history.push(second.clone(), None);
    gate.wait();
    settle(&mut history);
    assert_eq!(pop(&mut history).unwrap(), Some(second));
    assert!(history.is_empty());
}

#[test]
/// 验证读取已转存历史与新转存交错时栈仍一致。
fn undoing_spilled_entries_while_a_new_spill_runs_keeps_the_stack_consistent() {
    let mut history = History::with_limit(0);
    let first = entry(1);
    let second = entry(2);
    let third = entry(3);
    history.push(first.clone(), None);
    settle(&mut history);
    let gate = Arc::new(std::sync::Barrier::new(2));
    history.write_gate = Some(Arc::clone(&gate));
    history.push(second.clone(), None);
    assert_eq!(pop(&mut history).unwrap(), Some(second));
    assert_eq!(pop(&mut history).unwrap(), Some(first));
    history.push(third.clone(), None);
    gate.wait();
    settle(&mut history);
    assert_eq!(pop(&mut history).unwrap(), Some(third));
    assert!(history.is_empty());
}

#[test]
/// 验证磁盘写入失败后内存副本仍可撤销。
fn failed_write_keeps_the_memory_copy_undoable() {
    let mut history = History::with_limit(0);
    history.fail_next_write = true;
    let expected = entry(3);
    history.push(expected.clone(), None);
    settle(&mut history);
    assert!(history.error().is_some());
    assert_eq!(pop(&mut history).unwrap(), Some(expected));
    assert!(history.is_empty());
}

#[test]
/// 验证转存损坏会报告错误而不推进或丢弃历史。
fn damaged_spill_is_reported_without_discarding_the_history() {
    let mut history = History::with_limit(0);
    let expected = entry(4);
    history.push(expected.clone(), None);
    settle(&mut history);
    let file = history.file.as_ref().unwrap();
    let mut original = [0u8; 1];
    read_exact_at(file, &mut original, 0).unwrap();
    write_all_at(file, &[original[0] ^ 1], 0).unwrap();
    assert!(pop(&mut history).is_err());
    assert!(!history.is_empty());
    write_all_at(history.file.as_ref().unwrap(), &original, 0).unwrap();
    assert_eq!(pop(&mut history).unwrap(), Some(expected));
    assert!(history.is_empty());
}

#[test]
/// 验证转存保留浮点位模式，并释放匿名临时文件。
fn spill_preserves_float_bits_and_releases_its_temporary_file() {
    let mut bits = 0x1234_5678u32;
    let points: Vec<_> = (0..2_000)
        .map(|_| {
            bits = bits.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let x = f32::from_bits(bits);
            bits = bits.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            [x, f32::from_bits(bits)]
        })
        .collect();
    let expected: Vec<_> = points.iter().map(|point| point.map(f32::to_bits)).collect();
    let mark = Mark {
        ink: Ink::Red,
        weight: Weight::Medium,
        stroke_width: -0.0,
        kind: MarkKind::Pen(points),
    };
    let mut history = History::with_limit(0);
    history.push(
        Undo::Changed {
            index: 0,
            before: mark,
        },
        None,
    );
    settle(&mut history);
    let file = Arc::downgrade(history.file.as_ref().unwrap());
    let Undo::Changed { before, .. } = pop(&mut history).unwrap().unwrap() else {
        panic!("wrong undo variant")
    };
    assert_eq!(before.stroke_width.to_bits(), (-0.0f32).to_bits());
    let MarkKind::Pen(points) = before.kind else {
        panic!("wrong mark variant")
    };
    assert_eq!(
        points
            .iter()
            .map(|point| point.map(f32::to_bits))
            .collect::<Vec<_>>(),
        expected
    );
    history.clear();
    assert!(file.upgrade().is_none());
}

#[test]
#[ignore = "manual history spill performance measurement"]
/// 固定载荷、热缓存及单测试线程，比较同步读取成本、异步提交和完整完成耗时。
fn measure_spilled_history() {
    for count in [50_000usize, 1_000_000] {
        let source = Undo::Changed {
            index: 0,
            before: Mark {
                ink: Ink::Red,
                weight: Weight::Medium,
                stroke_width: 2.5,
                kind: MarkKind::Pen(
                    (0..count)
                        .map(|i| {
                            [
                                100.0 + (i % 300) as f32 * 0.25,
                                150.0 + (i % 170) as f32 * 0.25,
                            ]
                        })
                        .collect(),
                ),
            },
        };
        let mut reads = Vec::new();
        let mut writes = Vec::new();
        let mut blocking = Vec::new();
        let mut dispatch = Vec::new();
        for run in 0..23 {
            let mut history = History::with_limit(0);
            let start = Instant::now();
            history.push(source.clone(), None);
            settle(&mut history);
            let write = start.elapsed();
            let file = history.file.clone().unwrap();
            let end = history.disk_end;
            let baseline = || {
                let start = Instant::now();
                let (restored, _) = read_record(&file, end).unwrap();
                let elapsed = start.elapsed();
                assert_eq!(restored, source);
                elapsed
            };
            let before = run % 2 == 0;
            let mut synchronous = before.then(baseline);
            let start = Instant::now();
            assert!(matches!(history.pop(None).unwrap(), Pop::Pending));
            let submitted = start.elapsed();
            let recovered = pop(&mut history).unwrap().unwrap();
            let read = start.elapsed();
            if !before {
                synchronous = Some(baseline());
            }
            assert_eq!(recovered, source);
            if run >= 3 {
                reads.push(read);
                writes.push(write);
                blocking.push(synchronous.unwrap());
                dispatch.push(submitted);
            }
        }
        reads.sort_unstable();
        writes.sort_unstable();
        blocking.sort_unstable();
        dispatch.sort_unstable();
        println!(
            "{count} points, warmup=3 n=20 alternating order: spill p50 {:?}; sync read p50 {:?} p95 {:?}; async dispatch p50 {:?} p95 {:?}; async completion p50 {:?} p95 {:?}",
            writes[writes.len() / 2],
            blocking[10],
            blocking[18],
            dispatch[10],
            dispatch[18],
            reads[10],
            reads[18]
        );
    }
}
