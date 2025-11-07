use super::error::OcrError;
use super::utils::MINIMAL_SUPPORTED_TESSERACT_VERSION;
use std::collections::HashSet;

lazy_static::lazy_static! {
    pub static ref TESSERACT_SUPPORTED_LANGUAGE_CODES: HashSet<&'static str> = {
        let mut set = HashSet::new();
        set.insert("afr");
        set.insert("amh");
        set.insert("ara");
        set.insert("asm");
        set.insert("aze");
        set.insert("aze_cyrl");
        set.insert("bel");
        set.insert("ben");
        set.insert("bod");
        set.insert("bos");
        set.insert("bre");
        set.insert("bul");
        set.insert("cat");
        set.insert("ceb");
        set.insert("ces");
        set.insert("chi_sim");
        set.insert("chi_tra");
        set.insert("chr");
        set.insert("cos");
        set.insert("cym");
        set.insert("dan");
        set.insert("deu");
        set.insert("div");
        set.insert("dzo");
        set.insert("ell");
        set.insert("eng");
        set.insert("enm");
        set.insert("epo");
        set.insert("equ");
        set.insert("est");
        set.insert("eus");
        set.insert("fao");
        set.insert("fas");
        set.insert("fil");
        set.insert("fin");
        set.insert("fra");
        set.insert("frk");
        set.insert("frm");
        set.insert("fry");
        set.insert("gla");
        set.insert("gle");
        set.insert("glg");
        set.insert("grc");
        set.insert("guj");
        set.insert("hat");
        set.insert("heb");
        set.insert("hin");
        set.insert("hrv");
        set.insert("hun");
        set.insert("hye");
        set.insert("iku");
        set.insert("ind");
        set.insert("isl");
        set.insert("ita");
        set.insert("ita_old");
        set.insert("jav");
        set.insert("jpn");
        set.insert("kan");
        set.insert("kat");
        set.insert("kat_old");
        set.insert("kaz");
        set.insert("khm");
        set.insert("kir");
        set.insert("kmr");
        set.insert("kor");
        set.insert("lao");
        set.insert("lat");
        set.insert("lav");
        set.insert("lit");
        set.insert("ltz");
        set.insert("mal");
        set.insert("mar");
        set.insert("mkd");
        set.insert("mlt");
        set.insert("mon");
        set.insert("mri");
        set.insert("msa");
        set.insert("mya");
        set.insert("nep");
        set.insert("nld");
        set.insert("nor");
        set.insert("oci");
        set.insert("ori");
        set.insert("osd");
        set.insert("pan");
        set.insert("pol");
        set.insert("por");
        set.insert("pus");
        set.insert("que");
        set.insert("ron");
        set.insert("rus");
        set.insert("san");
        set.insert("sin");
        set.insert("slk");
        set.insert("slv");
        set.insert("snd");
        set.insert("spa");
        set.insert("spa_old");
        set.insert("sqi");
        set.insert("srp");
        set.insert("srp_latn");
        set.insert("sun");
        set.insert("swa");
        set.insert("swe");
        set.insert("syr");
        set.insert("tam");
        set.insert("tat");
        set.insert("tel");
        set.insert("tgk");
        set.insert("tha");
        set.insert("tir");
        set.insert("ton");
        set.insert("tur");
        set.insert("uig");
        set.insert("ukr");
        set.insert("urd");
        set.insert("uzb");
        set.insert("uzb_cyrl");
        set.insert("vie");
        set.insert("yid");
        set.insert("yor");
        set
    };
}

pub fn validate_language_code(lang_code: &str) -> Result<(), OcrError> {
    for code in lang_code.split('+') {
        if !TESSERACT_SUPPORTED_LANGUAGE_CODES.contains(code) {
            return Err(OcrError::InvalidLanguageCode(format!(
                "Language code '{}' is not supported by Tesseract",
                code
            )));
        }
    }
    Ok(())
}

pub fn validate_tesseract_version(version: u32) -> Result<(), OcrError> {
    if version < MINIMAL_SUPPORTED_TESSERACT_VERSION {
        return Err(OcrError::UnsupportedVersion(format!(
            "Tesseract version {} is not supported. Minimum required version is {}",
            version, MINIMAL_SUPPORTED_TESSERACT_VERSION
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_language_code_valid() {
        assert!(validate_language_code("eng").is_ok());
        assert!(validate_language_code("fra").is_ok());
        assert!(validate_language_code("deu").is_ok());
        assert!(validate_language_code("chi_sim").is_ok());
    }

    #[test]
    fn test_validate_language_code_multiple() {
        assert!(validate_language_code("eng+fra").is_ok());
        assert!(validate_language_code("eng+fra+deu").is_ok());
    }

    #[test]
    fn test_validate_language_code_invalid() {
        let result = validate_language_code("invalid_lang");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), OcrError::InvalidLanguageCode(_)));
    }

    #[test]
    fn test_validate_language_code_mixed_valid_invalid() {
        let result = validate_language_code("eng+invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_tesseract_version_valid() {
        assert!(validate_tesseract_version(5).is_ok());
        assert!(validate_tesseract_version(6).is_ok());
    }

    #[test]
    fn test_validate_tesseract_version_invalid() {
        let result = validate_tesseract_version(4);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), OcrError::UnsupportedVersion(_)));
    }

    #[test]
    fn test_language_codes_exist() {
        assert!(TESSERACT_SUPPORTED_LANGUAGE_CODES.contains("eng"));
        assert!(TESSERACT_SUPPORTED_LANGUAGE_CODES.contains("fra"));
        assert!(TESSERACT_SUPPORTED_LANGUAGE_CODES.contains("chi_sim"));
        assert!(!TESSERACT_SUPPORTED_LANGUAGE_CODES.contains("fake"));
    }
}
