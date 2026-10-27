use super::parse_styles;

#[test]
fn voice_files_hold_whole_style_vectors() {
    let bytes = vec![0u8; 256 * 4 * 3];
    assert_eq!(parse_styles(&bytes).unwrap().dim(), (3, 256));
}

#[test]
fn malformed_voice_files_are_rejected() {
    assert!(parse_styles(&[0, 0, 0]).is_err());
    assert!(parse_styles(&vec![0u8; 100 * 4]).is_err());
    assert!(parse_styles(&[]).is_err());
}
