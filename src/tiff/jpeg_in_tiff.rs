const APP14_SEGMENT_TRANSFORM_UNKNOWN: [u8; 16] = [
    0xff, 0xee, 0x00, 0x0e, 0x41, 0x64, 0x6f, 0x62, 0x65, 0x00, 0x64, 0x00, 0x00, 0x00, 0x00, 0x00,
];

pub(crate) fn set_app14_as_unknown(jpeg_tables: &mut Vec<u8>) {
    let (mut app14_ofs, mut dht_ofs) = (None, None);
    for (ofs, window) in jpeg_tables.windows(2).enumerate() {
        if window == [0xff, 0xee] {
            app14_ofs = Some(ofs);
            break;
        } else if window == [0xff, 0xc4] {
            dht_ofs = Some(ofs);
        }
    }
    match (app14_ofs, dht_ofs) {
        (Some(i), _) => {
            jpeg_tables[i + 16] = 0x00;
        }
        (None, Some(sos_ofs)) => {
            jpeg_tables.splice(
                sos_ofs..sos_ofs,
                APP14_SEGMENT_TRANSFORM_UNKNOWN.iter().copied(),
            );
        }
        _ => {}
    }
}
