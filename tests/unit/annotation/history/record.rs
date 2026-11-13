use super::*;

#[test]
/// 验证跨摘要读取块的大记录损坏仍被发现。
fn large_record_detects_corruption_across_hash_chunks() {
    let entry = Undo::PathChanged {
        index: 3,
        xor: vec![7; 200_000],
        stroke_xor: 5,
    };
    let spilled = spill(None, 0, &[(1, Arc::new(entry.clone()))]).unwrap();
    let end = spilled.records[0].1;
    for position in [0, 65_535, 65_536, 131_072] {
        let mut original = [0];
        read_exact_at(&spilled.file, &mut original, position).unwrap();
        write_all_at(&spilled.file, &[original[0] ^ 1], position).unwrap();
        assert_eq!(
            read_record(&spilled.file, end).unwrap_err(),
            "撤销记录校验失败"
        );
        write_all_at(&spilled.file, &original, position).unwrap();
    }
    assert_eq!(read_record(&spilled.file, end).unwrap().0, entry);
}

#[test]
/// 验证即使摘要正确，解码后剩余尾部字节仍被拒绝。
fn trailing_bytes_are_rejected_even_when_the_record_checksum_is_valid() {
    let mut bytes = rmp_serde::to_vec(&Undo::Added(3)).unwrap();
    bytes.push(0);
    let file = tempfile::tempfile().unwrap();
    write_all_at(&file, &bytes, 0).unwrap();
    let mut footer = [0; FOOTER as usize];
    footer[..8].copy_from_slice(&(bytes.len() as u64).to_le_bytes());
    footer[8..].copy_from_slice(blake3::hash(&bytes).as_bytes());
    write_all_at(&file, &footer, bytes.len() as u64).unwrap();
    assert_eq!(
        read_record(&file, bytes.len() as u64 + FOOTER).unwrap_err(),
        "撤销记录长度无效"
    );
}
