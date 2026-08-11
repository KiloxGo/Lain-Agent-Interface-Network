use crate::provider::{ModelError, ModelErrorKind};

const MAX_BUFFER_BYTES: usize = 1024 * 1024;

#[derive(Debug, Default)]
pub struct SseDecoder {
    buffer: Vec<u8>,
}

impl SseDecoder {
    pub fn push(&mut self, chunk: &[u8]) -> Result<Vec<String>, ModelError> {
        self.buffer.extend_from_slice(chunk);
        if self.buffer.len() > MAX_BUFFER_BYTES {
            return Err(ModelError::new(
                ModelErrorKind::Decode,
                "SSE event exceeded the one MiB decoder limit",
            ));
        }

        let mut events = Vec::new();
        while let Some((event_end, separator_len)) = find_separator(&self.buffer) {
            let event = self.buffer.drain(..event_end).collect::<Vec<_>>();
            self.buffer.drain(..separator_len);
            if let Some(data) = decode_event(&event)? {
                events.push(data);
            }
        }
        Ok(events)
    }

    pub fn has_pending_data(&self) -> bool {
        self.buffer.iter().any(|byte| !byte.is_ascii_whitespace())
    }
}

fn find_separator(buffer: &[u8]) -> Option<(usize, usize)> {
    for index in 0..buffer.len().saturating_sub(1) {
        if buffer[index..].starts_with(b"\r\n\r\n") {
            return Some((index, 4));
        }
        if buffer[index..].starts_with(b"\n\n") {
            return Some((index, 2));
        }
    }
    None
}

fn decode_event(event: &[u8]) -> Result<Option<String>, ModelError> {
    let event = std::str::from_utf8(event).map_err(|error| {
        ModelError::new(
            ModelErrorKind::Decode,
            format!("SSE event was not valid UTF-8: {error}"),
        )
    })?;
    let data = event
        .lines()
        .filter_map(|line| line.strip_prefix("data:"))
        .map(|line| line.strip_prefix(' ').unwrap_or(line))
        .collect::<Vec<_>>()
        .join("\n");
    Ok((!data.is_empty()).then_some(data))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_events_split_at_every_byte_boundary_including_unicode() {
        let payload = "data: {\"text\":\"你在这里\"}\r\n\r\ndata: [DONE]\n\n";
        for boundary in 1..payload.len() {
            let mut decoder = SseDecoder::default();
            let mut events = decoder
                .push(&payload.as_bytes()[..boundary])
                .expect("chunk");
            events.extend(
                decoder
                    .push(&payload.as_bytes()[boundary..])
                    .expect("remaining chunk"),
            );
            assert_eq!(events, vec!["{\"text\":\"你在这里\"}", "[DONE]"]);
            assert!(!decoder.has_pending_data());
        }
    }

    #[test]
    fn joins_multiple_data_lines_and_ignores_comments() {
        let mut decoder = SseDecoder::default();
        let events = decoder
            .push(b": keepalive\ndata: first\ndata: second\n\n")
            .expect("valid SSE");
        assert_eq!(events, vec!["first\nsecond"]);
    }
}
