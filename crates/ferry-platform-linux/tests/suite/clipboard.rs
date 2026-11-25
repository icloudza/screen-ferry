use ferry_platform::Clipboard;
use ferry_platform_linux::LinuxClipboard;
use ferry_types::ClipboardFormat;

#[test]
fn new_is_lazy_and_advertises_supported_formats() {
    let clipboard = LinuxClipboard::new();
    assert_eq!(
        clipboard.formats(),
        &[
            ClipboardFormat::TEXT,
            ClipboardFormat::HTML,
            ClipboardFormat::RTF,
            ClipboardFormat::PNG,
        ]
    );
}
