#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(non_camel_case_types)]
pub enum TessPageSegMode {
    PSM_OSD_ONLY = 0,
    PSM_AUTO_OSD = 1,
    PSM_AUTO_ONLY = 2,
    PSM_AUTO = 3,
    PSM_SINGLE_COLUMN = 4,
    PSM_SINGLE_BLOCK_VERT_TEXT = 5,
    PSM_SINGLE_BLOCK = 6,
    PSM_SINGLE_LINE = 7,
    PSM_SINGLE_WORD = 8,
    PSM_CIRCLE_WORD = 9,
    PSM_SINGLE_CHAR = 10,
    PSM_SPARSE_TEXT = 11,
    PSM_SPARSE_TEXT_OSD = 12,
    PSM_RAW_LINE = 13,
    PSM_COUNT = 14,
}

impl TessPageSegMode {
    pub fn from_int(value: i32) -> Self {
        match value {
            0 => TessPageSegMode::PSM_OSD_ONLY,
            1 => TessPageSegMode::PSM_AUTO_OSD,
            2 => TessPageSegMode::PSM_AUTO_ONLY,
            3 => TessPageSegMode::PSM_AUTO,
            4 => TessPageSegMode::PSM_SINGLE_COLUMN,
            5 => TessPageSegMode::PSM_SINGLE_BLOCK_VERT_TEXT,
            6 => TessPageSegMode::PSM_SINGLE_BLOCK,
            7 => TessPageSegMode::PSM_SINGLE_LINE,
            8 => TessPageSegMode::PSM_SINGLE_WORD,
            9 => TessPageSegMode::PSM_CIRCLE_WORD,
            10 => TessPageSegMode::PSM_SINGLE_CHAR,
            11 => TessPageSegMode::PSM_SPARSE_TEXT,
            12 => TessPageSegMode::PSM_SPARSE_TEXT_OSD,
            13 => TessPageSegMode::PSM_RAW_LINE,
            14 => TessPageSegMode::PSM_COUNT,
            _ => TessPageSegMode::PSM_AUTO,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(non_camel_case_types)]
pub enum TessPageIteratorLevel {
    RIL_BLOCK = 0,
    RIL_PARA = 1,
    RIL_TEXTLINE = 2,
    RIL_WORD = 3,
    RIL_SYMBOL = 4,
}

impl TessPageIteratorLevel {
    pub fn from_int(value: i32) -> Self {
        match value {
            0 => TessPageIteratorLevel::RIL_BLOCK,
            1 => TessPageIteratorLevel::RIL_PARA,
            2 => TessPageIteratorLevel::RIL_TEXTLINE,
            3 => TessPageIteratorLevel::RIL_WORD,
            4 => TessPageIteratorLevel::RIL_SYMBOL,
            _ => TessPageIteratorLevel::RIL_BLOCK,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(non_camel_case_types)]
pub enum TessPolyBlockType {
    PT_UNKNOWN = 0,
    PT_FLOWING_TEXT = 1,
    PT_HEADING_TEXT = 2,
    PT_PULLOUT_TEXT = 3,
    PT_EQUATION = 4,
    PT_INLINE_EQUATION = 5,
    PT_TABLE = 6,
    PT_VERTICAL_TEXT = 7,
    PT_CAPTION_TEXT = 8,
    PT_FLOWING_IMAGE = 9,
    PT_HEADING_IMAGE = 10,
    PT_PULLOUT_IMAGE = 11,
    PT_HORZ_LINE = 12,
    PT_VERT_LINE = 13,
    PT_NOISE = 14,
    PT_COUNT = 15,
}

impl TessPolyBlockType {
    pub fn from_int(value: i32) -> Self {
        match value {
            0 => TessPolyBlockType::PT_UNKNOWN,
            1 => TessPolyBlockType::PT_FLOWING_TEXT,
            2 => TessPolyBlockType::PT_HEADING_TEXT,
            3 => TessPolyBlockType::PT_PULLOUT_TEXT,
            4 => TessPolyBlockType::PT_EQUATION,
            5 => TessPolyBlockType::PT_INLINE_EQUATION,
            6 => TessPolyBlockType::PT_TABLE,
            7 => TessPolyBlockType::PT_VERTICAL_TEXT,
            8 => TessPolyBlockType::PT_CAPTION_TEXT,
            9 => TessPolyBlockType::PT_FLOWING_IMAGE,
            10 => TessPolyBlockType::PT_HEADING_IMAGE,
            11 => TessPolyBlockType::PT_PULLOUT_IMAGE,
            12 => TessPolyBlockType::PT_HORZ_LINE,
            13 => TessPolyBlockType::PT_VERT_LINE,
            14 => TessPolyBlockType::PT_NOISE,
            15 => TessPolyBlockType::PT_COUNT,
            _ => TessPolyBlockType::PT_UNKNOWN,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(non_camel_case_types)]
pub enum TessOrientation {
    ORIENTATION_PAGE_UP = 0,
    ORIENTATION_PAGE_RIGHT = 1,
    ORIENTATION_PAGE_DOWN = 2,
    ORIENTATION_PAGE_LEFT = 3,
}

impl TessOrientation {
    pub fn from_int(value: i32) -> Self {
        match value {
            0 => TessOrientation::ORIENTATION_PAGE_UP,
            1 => TessOrientation::ORIENTATION_PAGE_RIGHT,
            2 => TessOrientation::ORIENTATION_PAGE_DOWN,
            3 => TessOrientation::ORIENTATION_PAGE_LEFT,
            _ => TessOrientation::ORIENTATION_PAGE_UP,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(non_camel_case_types)]
pub enum TessParagraphJustification {
    JUSTIFICATION_UNKNOWN = 0,
    JUSTIFICATION_LEFT = 1,
    JUSTIFICATION_CENTER = 2,
    JUSTIFICATION_RIGHT = 3,
}

impl TessParagraphJustification {
    pub fn from_int(value: i32) -> Self {
        match value {
            0 => TessParagraphJustification::JUSTIFICATION_UNKNOWN,
            1 => TessParagraphJustification::JUSTIFICATION_LEFT,
            2 => TessParagraphJustification::JUSTIFICATION_CENTER,
            3 => TessParagraphJustification::JUSTIFICATION_RIGHT,
            _ => TessParagraphJustification::JUSTIFICATION_UNKNOWN,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(non_camel_case_types)]
pub enum TessWritingDirection {
    WRITING_DIRECTION_LEFT_TO_RIGHT = 0,
    WRITING_DIRECTION_RIGHT_TO_LEFT = 1,
    WRITING_DIRECTION_TOP_TO_BOTTOM = 2,
}

impl TessWritingDirection {
    pub fn from_int(value: i32) -> Self {
        match value {
            0 => TessWritingDirection::WRITING_DIRECTION_LEFT_TO_RIGHT,
            1 => TessWritingDirection::WRITING_DIRECTION_RIGHT_TO_LEFT,
            2 => TessWritingDirection::WRITING_DIRECTION_TOP_TO_BOTTOM,
            _ => TessWritingDirection::WRITING_DIRECTION_LEFT_TO_RIGHT,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(non_camel_case_types)]
pub enum TessTextlineOrder {
    TEXTLINE_ORDER_LEFT_TO_RIGHT = 0,
    TEXTLINE_ORDER_RIGHT_TO_LEFT = 1,
    TEXTLINE_ORDER_TOP_TO_BOTTOM = 2,
}

impl TessTextlineOrder {
    pub fn from_int(value: i32) -> Self {
        match value {
            0 => TessTextlineOrder::TEXTLINE_ORDER_LEFT_TO_RIGHT,
            1 => TessTextlineOrder::TEXTLINE_ORDER_RIGHT_TO_LEFT,
            2 => TessTextlineOrder::TEXTLINE_ORDER_TOP_TO_BOTTOM,
            _ => TessTextlineOrder::TEXTLINE_ORDER_LEFT_TO_RIGHT,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_seg_mode_from_int() {
        assert_eq!(TessPageSegMode::from_int(0), TessPageSegMode::PSM_OSD_ONLY);
        assert_eq!(TessPageSegMode::from_int(3), TessPageSegMode::PSM_AUTO);
        assert_eq!(TessPageSegMode::from_int(10), TessPageSegMode::PSM_SINGLE_CHAR);
        assert_eq!(TessPageSegMode::from_int(999), TessPageSegMode::PSM_AUTO);
    }

    #[test]
    fn test_page_seg_mode_conversion() {
        let mode = TessPageSegMode::PSM_SINGLE_LINE;
        assert_eq!(mode as i32, 7);
    }

    #[test]
    fn test_page_iterator_level_from_int() {
        assert_eq!(TessPageIteratorLevel::from_int(0), TessPageIteratorLevel::RIL_BLOCK);
        assert_eq!(TessPageIteratorLevel::from_int(3), TessPageIteratorLevel::RIL_WORD);
        assert_eq!(TessPageIteratorLevel::from_int(-1), TessPageIteratorLevel::RIL_BLOCK);
    }

    #[test]
    fn test_poly_block_type_from_int() {
        assert_eq!(TessPolyBlockType::from_int(1), TessPolyBlockType::PT_FLOWING_TEXT);
        assert_eq!(TessPolyBlockType::from_int(6), TessPolyBlockType::PT_TABLE);
        assert_eq!(TessPolyBlockType::from_int(100), TessPolyBlockType::PT_UNKNOWN);
    }

    #[test]
    fn test_orientation_from_int() {
        assert_eq!(TessOrientation::from_int(0), TessOrientation::ORIENTATION_PAGE_UP);
        assert_eq!(TessOrientation::from_int(2), TessOrientation::ORIENTATION_PAGE_DOWN);
        assert_eq!(TessOrientation::from_int(5), TessOrientation::ORIENTATION_PAGE_UP);
    }

    #[test]
    fn test_paragraph_justification_from_int() {
        assert_eq!(
            TessParagraphJustification::from_int(1),
            TessParagraphJustification::JUSTIFICATION_LEFT
        );
        assert_eq!(
            TessParagraphJustification::from_int(3),
            TessParagraphJustification::JUSTIFICATION_RIGHT
        );
        assert_eq!(
            TessParagraphJustification::from_int(-1),
            TessParagraphJustification::JUSTIFICATION_UNKNOWN
        );
    }

    #[test]
    fn test_writing_direction_from_int() {
        assert_eq!(
            TessWritingDirection::from_int(0),
            TessWritingDirection::WRITING_DIRECTION_LEFT_TO_RIGHT
        );
        assert_eq!(
            TessWritingDirection::from_int(1),
            TessWritingDirection::WRITING_DIRECTION_RIGHT_TO_LEFT
        );
        assert_eq!(
            TessWritingDirection::from_int(10),
            TessWritingDirection::WRITING_DIRECTION_LEFT_TO_RIGHT
        );
    }

    #[test]
    fn test_textline_order_from_int() {
        assert_eq!(
            TessTextlineOrder::from_int(0),
            TessTextlineOrder::TEXTLINE_ORDER_LEFT_TO_RIGHT
        );
        assert_eq!(
            TessTextlineOrder::from_int(2),
            TessTextlineOrder::TEXTLINE_ORDER_TOP_TO_BOTTOM
        );
        assert_eq!(
            TessTextlineOrder::from_int(99),
            TessTextlineOrder::TEXTLINE_ORDER_LEFT_TO_RIGHT
        );
    }

    #[test]
    fn test_enums_are_copy() {
        fn assert_copy<T: Copy>() {}
        assert_copy::<TessPageSegMode>();
        assert_copy::<TessPageIteratorLevel>();
        assert_copy::<TessPolyBlockType>();
        assert_copy::<TessOrientation>();
        assert_copy::<TessParagraphJustification>();
        assert_copy::<TessWritingDirection>();
        assert_copy::<TessTextlineOrder>();
    }
}
