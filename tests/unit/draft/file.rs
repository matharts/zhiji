use super::*;

#[test]
/// 验证缓冲恰好达到上限可写，额外字节被拒绝。
fn bounded_buffer_accepts_its_limit_and_rejects_extra_bytes() {
    let mut output = BoundedBuffer {
        bytes: Vec::new(),
        limit: 5,
        exceeded: false,
    };
    output.write_all(b"abc").unwrap();
    output.write_all(b"de").unwrap();
    assert_eq!(output.bytes, b"abcde");
    assert!(!output.exceeded);
    assert!(output.write_all(b"f").is_err());
    assert!(output.exceeded);
    assert_eq!(output.bytes, b"abcde");
}
