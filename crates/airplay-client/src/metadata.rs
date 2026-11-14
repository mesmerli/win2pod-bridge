//! AirPlay 2 DMAP metadata encoding.
//!
//! Provides DMAP binary tagging (`application/x-dmap-tagged`) for publishing
//! current track metadata (`minm`, `asar`, `asal`) to AirPlay 2 receivers.
//! When a HomePod receives track metadata, it enables touch surface gestures
//! for skipping tracks (double-tap = Next, triple-tap = Previous).

/// Appends a DMAP tagged TLV (tag, 32-bit big-endian length, UTF-8 payload) to the buffer.
fn append_dmap_chunk(buf: &mut Vec<u8>, tag: &[u8; 4], text: &str) {
    let bytes = text.as_bytes();
    buf.extend_from_slice(tag);
    buf.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
    buf.extend_from_slice(bytes);
}

/// Encodes track metadata into an `mlit` DMAP container.
///
/// If title is empty, a fallback placeholder is used to ensure the receiver
/// recognizes the stream as playing a track rather than an unskippable live feed.
pub fn encode_dmap_metadata(title: &str, artist: &str, album: &str) -> Vec<u8> {
    let effective_title = if title.trim().is_empty() {
        "Win2Pod Stream"
    } else {
        title.trim()
    };
    let effective_artist = if artist.trim().is_empty() {
        "Windows PC"
    } else {
        artist.trim()
    };
    let effective_album = if album.trim().is_empty() {
        "Win2Pod"
    } else {
        album.trim()
    };

    let mut inner = Vec::new();
    append_dmap_chunk(&mut inner, b"minm", effective_title);
    append_dmap_chunk(&mut inner, b"asar", effective_artist);
    append_dmap_chunk(&mut inner, b"asal", effective_album);

    let mut out = Vec::with_capacity(8 + inner.len());
    out.extend_from_slice(b"mlit");
    out.extend_from_slice(&(inner.len() as u32).to_be_bytes());
    out.extend_from_slice(&inner);
    out
}

/// Encodes a single MediaRemote command info structure into binary plist bytes.
fn encode_command_info(cmd_id: u32) -> Result<Vec<u8>, plist::Error> {
    let mut dict = plist::Dictionary::new();
    dict.insert("command".into(), plist::Value::Integer((cmd_id as i64).into()));
    dict.insert("kCommandInfoCommandKey".into(), plist::Value::Integer((cmd_id as i64).into()));
    dict.insert("enabled".into(), plist::Value::Boolean(true));
    dict.insert("kCommandInfoEnabledKey".into(), plist::Value::Boolean(true));

    let mut buf = Vec::new();
    plist::to_writer_binary(&mut buf, &plist::Value::Dictionary(dict))?;
    Ok(buf)
}

/// Builds an `updateMRSupportedCommands` binary plist payload to declare supported playback commands
/// (Play, Pause, Toggle, Stop, Next, Previous) to the AirPlay receiver.
pub fn build_supported_commands_plist() -> Result<Vec<u8>, plist::Error> {
    let mut command_datas = Vec::new();
    // 0: Play, 1: Pause, 2: TogglePlayPause, 3: Stop, 4: NextTrack, 5: PreviousTrack
    for cmd_id in [0, 1, 2, 3, 4, 5] {
        let data = encode_command_info(cmd_id)?;
        command_datas.push(plist::Value::Data(data));
    }

    let mut params = plist::Dictionary::new();
    params.insert(
        "mrSupportedCommandsFromSender".into(),
        plist::Value::Array(command_datas),
    );

    let mut root = plist::Dictionary::new();
    root.insert(
        "type".into(),
        plist::Value::String("updateMRSupportedCommands".into()),
    );
    root.insert("params".into(), plist::Value::Dictionary(params));

    let mut buf = Vec::new();
    plist::to_writer_binary(&mut buf, &plist::Value::Dictionary(root))?;
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_dmap_metadata() {
        let title = "Night Dancer";
        let artist = "imase";
        let album = "POP DROP";

        let dmap = encode_dmap_metadata(title, artist, album);
        assert!(dmap.len() >= 8);
        assert_eq!(&dmap[0..4], b"mlit");

        let total_len = u32::from_be_bytes([dmap[4], dmap[5], dmap[6], dmap[7]]) as usize;
        assert_eq!(dmap.len(), 8 + total_len);

        // Check minm tag
        let mut offset = 8;
        assert_eq!(&dmap[offset..offset + 4], b"minm");
        let title_len = u32::from_be_bytes([
            dmap[offset + 4],
            dmap[offset + 5],
            dmap[offset + 6],
            dmap[offset + 7],
        ]) as usize;
        offset += 8;
        assert_eq!(&dmap[offset..offset + title_len], title.as_bytes());
        offset += title_len;

        // Check asar tag
        assert_eq!(&dmap[offset..offset + 4], b"asar");
        let artist_len = u32::from_be_bytes([
            dmap[offset + 4],
            dmap[offset + 5],
            dmap[offset + 6],
            dmap[offset + 7],
        ]) as usize;
        offset += 8;
        assert_eq!(&dmap[offset..offset + artist_len], artist.as_bytes());
        offset += artist_len;

        // Check asal tag
        assert_eq!(&dmap[offset..offset + 4], b"asal");
        let album_len = u32::from_be_bytes([
            dmap[offset + 4],
            dmap[offset + 5],
            dmap[offset + 6],
            dmap[offset + 7],
        ]) as usize;
        offset += 8;
        assert_eq!(&dmap[offset..offset + album_len], album.as_bytes());
        offset += album_len;

        assert_eq!(offset, dmap.len());
    }

    #[test]
    fn test_encode_dmap_metadata_fallback() {
        let dmap = encode_dmap_metadata("", "", "");
        assert_eq!(&dmap[0..4], b"mlit");
        let text = String::from_utf8_lossy(&dmap);
        assert!(text.contains("Win2Pod Stream"));
        assert!(text.contains("Windows PC"));
        assert!(text.contains("Win2Pod"));
    }
}
