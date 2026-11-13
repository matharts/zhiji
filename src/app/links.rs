//! 系统菜单与设置页共用的帮助、反馈和环境信息。

use rune::Tx;

/// 返回反馈和剪贴板共用的版本及运行环境，不读取用户会话。
pub(crate) fn app_info() -> String {
    let system = match std::env::consts::OS {
        "macos" => "macOS",
        "windows" => "Windows",
        "linux" => "Linux",
        other => other,
    };
    format!(
        "知几 / Zhiji\n版本：{}\n操作系统：{system}\n运行架构：{}",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::ARCH,
    )
}

/// 构造待用户填写并提交的问题；仅预填描述框，不自动发送。
fn feedback_url() -> String {
    let body = format!(
        "## 问题描述\n\n\n## 复现步骤\n\n1. \n\n## 预期结果\n\n\n## 环境信息\n\n```text\n{}\n```\n",
        app_info(),
    );
    let mut url = String::from("https://github.com/matharts/zhiji/issues/new?body=");
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    // 对 UTF-8 字节编码，换行、中文和 Markdown 标记都不能成为查询分隔符。
    for byte in body.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
            url.push(char::from(byte));
        } else {
            url.push('%');
            url.push(char::from(HEX[(byte >> 4) as usize]));
            url.push(char::from(HEX[(byte & 15) as usize]));
        }
    }
    url
}

/// 在默认浏览器中打开已预填环境信息的新建问题页。
pub(crate) fn open_feedback(tx: &mut Tx) {
    tx.open_url(&feedback_url())
        .expect("encoded issue URL is valid");
}
