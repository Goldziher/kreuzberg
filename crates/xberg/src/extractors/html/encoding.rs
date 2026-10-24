use encoding_rs::Encoding;

const PRESCAN_BYTES: usize = 1024;

pub(super) fn declared_html_encoding(content: &[u8]) -> Option<&'static Encoding> {
    if let Some((encoding, _)) = Encoding::for_bom(content) {
        return Some(encoding);
    }
    Prescan {
        bytes: &content[..content.len().min(PRESCAN_BYTES)],
        position: 0,
    }
    .encoding()
}

struct Prescan<'a> {
    bytes: &'a [u8],
    position: usize,
}

enum Attribute<'a> {
    Parsed(&'a [u8], &'a [u8]),
    TagEnd,
}

impl<'a> Prescan<'a> {
    fn encoding(&mut self) -> Option<&'static Encoding> {
        while let Some(&byte) = self.bytes.get(self.position) {
            let remaining = &self.bytes[self.position..];
            if remaining.starts_with(b"<!--") {
                self.position += 2;
                self.skip_through(b"-->")?;
                continue;
            }
            if self.is_meta() {
                self.position += b"<meta".len();
                if let Some(encoding) = self.meta_encoding() {
                    return Some(encoding);
                }
            } else if self.is_tag() {
                self.skip_tag();
            } else if byte == b'<' && matches!(remaining.get(1), Some(b'!' | b'/' | b'?')) {
                self.skip_through(b">")?;
                continue;
            }
            self.position += 1;
        }
        None
    }

    fn is_meta(&self) -> bool {
        let remaining = &self.bytes[self.position..];
        remaining
            .get(..b"<meta".len())
            .is_some_and(|name| name.eq_ignore_ascii_case(b"<meta"))
            && remaining
                .get(b"<meta".len())
                .is_some_and(|&byte| html_space(byte) || byte == b'/')
    }

    fn is_tag(&self) -> bool {
        let remaining = &self.bytes[self.position..];
        let Some(after_open) = remaining.strip_prefix(b"<") else {
            return false;
        };
        after_open
            .strip_prefix(b"/")
            .unwrap_or(after_open)
            .first()
            .is_some_and(u8::is_ascii_alphabetic)
    }

    fn skip_through(&mut self, end: &[u8]) -> Option<()> {
        let offset = self.bytes[self.position..]
            .windows(end.len())
            .position(|part| part == end)?;
        self.position += offset + end.len();
        Some(())
    }

    fn skip_tag(&mut self) {
        while self
            .bytes
            .get(self.position)
            .is_some_and(|&byte| !html_space(byte) && byte != b'>')
        {
            self.position += 1;
        }
        while let Some(Attribute::Parsed(..)) = self.attribute() {}
    }

    fn meta_encoding(&mut self) -> Option<&'static Encoding> {
        let mut attributes = MetaAttributes::default();
        loop {
            match self.attribute()? {
                Attribute::Parsed(name, value) => attributes.process(name, value),
                Attribute::TagEnd => return attributes.encoding(),
            }
        }
    }

    fn attribute(&mut self) -> Option<Attribute<'a>> {
        while self
            .bytes
            .get(self.position)
            .is_some_and(|&byte| html_space(byte) || byte == b'/')
        {
            self.position += 1;
        }
        if *self.bytes.get(self.position)? == b'>' {
            return Some(Attribute::TagEnd);
        }
        let start = self.position;
        loop {
            let byte = *self.bytes.get(self.position)?;
            if html_space(byte) || matches!(byte, b'/' | b'>') || (byte == b'=' && self.position > start) {
                break;
            }
            self.position += 1;
        }
        let name = &self.bytes[start..self.position];
        self.skip_spaces()?;
        if *self.bytes.get(self.position)? != b'=' {
            return Some(Attribute::Parsed(name, b""));
        }
        self.position += 1;
        Some(Attribute::Parsed(name, self.attribute_value()?))
    }

    fn skip_spaces(&mut self) -> Option<()> {
        while html_space(*self.bytes.get(self.position)?) {
            self.position += 1;
        }
        Some(())
    }

    fn attribute_value(&mut self) -> Option<&'a [u8]> {
        self.skip_spaces()?;
        let first = *self.bytes.get(self.position)?;
        if matches!(first, b'\'' | b'"') {
            self.position += 1;
            let start = self.position;
            let Some(length) = self.bytes[start..].iter().position(|&byte| byte == first) else {
                self.position = self.bytes.len();
                return None;
            };
            self.position += length + 1;
            // The prescan end condition applies immediately after the closing quote too. ~keep
            self.bytes.get(self.position)?;
            return Some(&self.bytes[start..start + length]);
        }
        let start = self.position;
        while !html_space(*self.bytes.get(self.position)?) && self.bytes[self.position] != b'>' {
            self.position += 1;
        }
        Some(&self.bytes[start..self.position])
    }
}

#[derive(Default)]
enum Charset {
    #[default]
    Unset,
    Unsupported,
    Known(&'static Encoding),
}

#[derive(Default)]
struct MetaAttributes {
    charset: Charset,
    needs_pragma: bool,
    got_pragma: bool,
    charset_seen: bool,
    content_seen: bool,
    pragma_seen: bool,
}

impl MetaAttributes {
    // An unsupported direct label still overrides content, in either attribute order.
    // https://html.spec.whatwg.org/multipage/parsing.html#prescan-a-byte-stream-to-determine-its-encoding ~keep
    fn process(&mut self, name: &[u8], value: &[u8]) {
        if name.eq_ignore_ascii_case(b"http-equiv") && !self.pragma_seen {
            self.pragma_seen = true;
            self.got_pragma = value.eq_ignore_ascii_case(b"content-type");
        } else if name.eq_ignore_ascii_case(b"content") && !self.content_seen {
            self.content_seen = true;
            if matches!(self.charset, Charset::Unset)
                && let Some(encoding) = content_encoding(value)
            {
                self.charset = Charset::Known(encoding);
                self.needs_pragma = true;
            }
        } else if name.eq_ignore_ascii_case(b"charset") && !self.charset_seen {
            self.charset_seen = true;
            self.charset = Encoding::for_label(value).map_or(Charset::Unsupported, Charset::Known);
            self.needs_pragma = false;
        }
    }

    fn encoding(&self) -> Option<&'static Encoding> {
        let Charset::Known(encoding) = self.charset else {
            return None;
        };
        if self.needs_pragma && !self.got_pragma {
            return None;
        }
        Some(
            if encoding == encoding_rs::UTF_16LE || encoding == encoding_rs::UTF_16BE {
                encoding_rs::UTF_8
            } else if encoding == encoding_rs::X_USER_DEFINED {
                encoding_rs::WINDOWS_1252
            } else {
                encoding
            },
        )
    }
}

fn content_encoding(mut value: &[u8]) -> Option<&'static Encoding> {
    loop {
        let position = value
            .windows(b"charset".len())
            .position(|part| part.eq_ignore_ascii_case(b"charset"))?;
        value = trim_html_spaces(&value[position + b"charset".len()..]);
        if let Some(after_equals) = value.strip_prefix(b"=") {
            value = trim_html_spaces(after_equals);
            break;
        }
    }
    let first = *value.first()?;
    let label = if matches!(first, b'\'' | b'"') {
        let quoted = &value[1..];
        &quoted[..quoted.iter().position(|&byte| byte == first)?]
    } else {
        let end = value
            .iter()
            .position(|&byte| html_space(byte) || byte == b';')
            .unwrap_or(value.len());
        &value[..end]
    };
    Encoding::for_label(label)
}

fn trim_html_spaces(bytes: &[u8]) -> &[u8] {
    let end = bytes.iter().position(|&byte| !html_space(byte)).unwrap_or(bytes.len());
    &bytes[end..]
}

fn html_space(byte: u8) -> bool {
    matches!(byte, b'\t' | b'\n' | b'\x0c' | b'\r' | b' ')
}
