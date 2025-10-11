use super::tsv_parser::TSVWord;

pub fn create_test_word(left: u32, top: u32, width: u32, height: u32, text: &str) -> TSVWord {
    TSVWord {
        level: 5,
        page_num: 1,
        block_num: 0,
        par_num: 0,
        line_num: 0,
        word_num: 0,
        left,
        top,
        width,
        height,
        conf: 95.0,
        text: text.to_string(),
    }
}
