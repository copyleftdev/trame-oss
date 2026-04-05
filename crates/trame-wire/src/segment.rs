//! Zero-copy X12 segment representation.
//!
//! A [`Segment`] is a lightweight view into the source byte buffer. It holds
//! no owned data — just a `&[u8]` slice and the delimiter bytes needed to
//! split the segment into elements.

/// Iterator over the elements of an X12 segment.
#[derive(Debug, Clone)]
pub struct ElementIter<'a> {
    remaining: &'a [u8],
    element_sep: u8,
    done: bool,
}

impl<'a> Iterator for ElementIter<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        if let Some(pos) = memchr_byte(self.element_sep, self.remaining) {
            let element = &self.remaining[..pos];
            self.remaining = &self.remaining[pos + 1..];
            Some(element)
        } else {
            self.done = true;
            Some(self.remaining)
        }
    }
}

/// Iterator over sub-elements (components) within a single element.
#[derive(Debug, Clone)]
pub struct SubElementIter<'a> {
    remaining: &'a [u8],
    sub_sep: u8,
    done: bool,
}

impl<'a> Iterator for SubElementIter<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        if let Some(pos) = memchr_byte(self.sub_sep, self.remaining) {
            let part = &self.remaining[..pos];
            self.remaining = &self.remaining[pos + 1..];
            Some(part)
        } else {
            self.done = true;
            Some(self.remaining)
        }
    }
}

/// A single X12 segment — zero-copy reference into the source buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Segment<'a> {
    /// The raw bytes of this segment (excluding terminator).
    raw: &'a [u8],
    /// Element separator byte.
    element_sep: u8,
    /// Sub-element separator byte.
    sub_element_sep: u8,
}

impl<'a> Segment<'a> {
    /// Create a new segment from raw bytes and delimiters.
    ///
    /// The `raw` slice must NOT include the segment terminator.
    pub fn new(raw: &'a [u8], element_sep: u8, sub_element_sep: u8) -> Self {
        Self {
            raw,
            element_sep,
            sub_element_sep,
        }
    }

    /// The segment identifier (first element, e.g., `b"ISA"`, `b"GS"`, `b"ST"`).
    pub fn id(&self) -> &'a [u8] {
        match memchr_byte(self.element_sep, self.raw) {
            Some(pos) => &self.raw[..pos],
            None => self.raw,
        }
    }

    /// The segment identifier as a `&str`.
    ///
    /// Returns `None` if the segment ID is not valid UTF-8.
    /// In practice, X12 segment identifiers are always ASCII.
    pub fn id_str(&self) -> Option<&str> {
        std::str::from_utf8(self.id()).ok()
    }

    /// Get an element by 0-based index (index 0 is the segment identifier).
    pub fn element(&self, index: usize) -> Option<&'a [u8]> {
        self.elements().nth(index)
    }

    /// Get an element as a `&str` by 0-based index.
    ///
    /// Returns `None` if the index is out of bounds or the element is not valid UTF-8.
    pub fn element_str(&self, index: usize) -> Option<&'a str> {
        self.element(index)
            .and_then(|e| std::str::from_utf8(e).ok())
    }

    /// The number of elements in this segment (including the segment ID).
    pub fn element_count(&self) -> usize {
        self.elements().count()
    }

    /// Iterate over sub-elements of the element at `index`.
    ///
    /// Returns `None` if the index is out of bounds.
    pub fn sub_elements(&self, index: usize) -> Option<SubElementIter<'a>> {
        self.element(index).map(|e| SubElementIter {
            remaining: e,
            sub_sep: self.sub_element_sep,
            done: false,
        })
    }

    /// The raw bytes of this segment (excluding terminator).
    pub fn raw(&self) -> &'a [u8] {
        self.raw
    }

    /// Iterate over all elements of this segment.
    pub fn elements(&self) -> ElementIter<'a> {
        ElementIter {
            remaining: self.raw,
            element_sep: self.element_sep,
            done: false,
        }
    }
}

/// Find the first occurrence of `needle` in `haystack`.
///
/// This is a simple linear scan — no external dependency needed.
#[inline]
fn memchr_byte(needle: u8, haystack: &[u8]) -> Option<usize> {
    haystack.iter().position(|&b| b == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_segment(raw: &[u8]) -> Segment<'_> {
        Segment::new(raw, b'*', b':')
    }

    #[test]
    fn segment_id() {
        let seg = test_segment(b"ISA*00*          ");
        assert_eq!(seg.id(), b"ISA");
        assert_eq!(seg.id_str().unwrap(), "ISA");
    }

    #[test]
    fn segment_id_no_elements() {
        let seg = test_segment(b"SE");
        assert_eq!(seg.id(), b"SE");
    }

    #[test]
    fn element_access() {
        let seg = test_segment(b"GS*HP*SENDER*RECEIVER*20210901*1234*1*X*005010X222A1");
        assert_eq!(seg.element(0), Some(b"GS".as_ref()));
        assert_eq!(seg.element(1), Some(b"HP".as_ref()));
        assert_eq!(seg.element(2), Some(b"SENDER".as_ref()));
        assert_eq!(seg.element(8), Some(b"005010X222A1".as_ref()));
        assert_eq!(seg.element(9), None);
    }

    #[test]
    fn element_str() {
        let seg = test_segment(b"ST*837*0001*005010X222A1");
        assert_eq!(seg.element_str(0), Some("ST"));
        assert_eq!(seg.element_str(1), Some("837"));
        assert_eq!(seg.element_str(2), Some("0001"));
        assert_eq!(seg.element_str(3), Some("005010X222A1"));
        assert_eq!(seg.element_str(4), None);
    }

    #[test]
    fn element_count() {
        let seg = test_segment(b"CLM*12345*100***11:B:1*Y*A*Y*I");
        // CLM + 9 data elements = 10 total
        assert_eq!(seg.element_count(), 10);
    }

    #[test]
    fn empty_elements() {
        let seg = test_segment(b"SV1***25");
        assert_eq!(seg.element(0), Some(b"SV1".as_ref()));
        assert_eq!(seg.element(1), Some(b"".as_ref()));
        assert_eq!(seg.element(2), Some(b"".as_ref()));
        assert_eq!(seg.element(3), Some(b"25".as_ref()));
    }

    #[test]
    fn sub_elements() {
        let seg = test_segment(b"SV1*HC:99213:25*100*UN*1");
        let subs: Vec<&[u8]> = seg.sub_elements(1).unwrap().collect();
        assert_eq!(subs, vec![b"HC".as_ref(), b"99213".as_ref(), b"25".as_ref()]);
    }

    #[test]
    fn sub_elements_no_components() {
        let seg = test_segment(b"NM1*IL*1*DOE*JOHN");
        let subs: Vec<&[u8]> = seg.sub_elements(1).unwrap().collect();
        // No sub-element separator, so the whole element is one sub-element
        assert_eq!(subs, vec![b"IL".as_ref()]);
    }

    #[test]
    fn sub_elements_out_of_bounds() {
        let seg = test_segment(b"SE*5*0001");
        assert!(seg.sub_elements(10).is_none());
    }

    #[test]
    fn raw_bytes() {
        let raw = b"ISA*00*TEST";
        let seg = test_segment(raw);
        assert_eq!(seg.raw(), raw);
    }

    #[test]
    fn iterate_elements() {
        let seg = test_segment(b"GE*1*1234");
        let elems: Vec<&[u8]> = seg.elements().collect();
        assert_eq!(
            elems,
            vec![b"GE".as_ref(), b"1".as_ref(), b"1234".as_ref()]
        );
    }

    #[test]
    fn consecutive_separators() {
        // Three consecutive separators -> three empty elements between them
        let seg = test_segment(b"AAA***");
        let elems: Vec<&[u8]> = seg.elements().collect();
        assert_eq!(
            elems,
            vec![
                b"AAA".as_ref(),
                b"".as_ref(),
                b"".as_ref(),
                b"".as_ref(),
            ]
        );
    }
}
