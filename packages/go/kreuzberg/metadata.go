package kreuzberg

import "encoding/json"

var metadataCoreKeys = map[string]struct{}{
	"language":            {},
	"date":                {},
	"subject":             {},
	"format_type":         {},
	"image_preprocessing": {},
	"json_schema":         {},
	"error":               {},
}

var formatFieldSets = map[string][]string{
	"pdf": {
		"title", "subject", "authors", "keywords", "created_at", "modified_at",
		"created_by", "producer", "page_count", "pdf_version", "is_encrypted",
		"width", "height", "summary",
	},
	"excel":   {"sheet_count", "sheet_names"},
	"email":   {"from_email", "from_name", "to_emails", "cc_emails", "bcc_emails", "message_id", "attachments"},
	"pptx":    {"title", "author", "description", "summary", "fonts"},
	"archive": {"format", "file_count", "file_list", "total_size", "compressed_size"},
	"image":   {"width", "height", "format", "exif"},
	"xml":     {"element_count", "unique_elements"},
	"text":    {"line_count", "word_count", "character_count", "headers", "links", "code_blocks"},
	"html": {
		"title", "description", "keywords", "author", "canonical", "base_href",
		"og_title", "og_description", "og_image", "og_url", "og_type", "og_site_name",
		"twitter_card", "twitter_title", "twitter_description", "twitter_image", "twitter_site", "twitter_creator",
		"link_author", "link_license", "link_alternate",
	},
	"ocr": {"language", "psm", "output_format", "table_count", "table_rows", "table_cols"},
}

// UnmarshalJSON ensures Metadata captures flattened format unions and additional custom fields.
func (m *Metadata) UnmarshalJSON(data []byte) error {
	var aux struct {
		Language           *string                     `json:"language"`
		Date               *string                     `json:"date"`
		Subject            *string                     `json:"subject"`
		FormatType         string                      `json:"format_type"`
		ImagePreprocessing *ImagePreprocessingMetadata `json:"image_preprocessing"`
		JSONSchema         json.RawMessage             `json:"json_schema"`
		Error              *ErrorMetadata              `json:"error"`
	}

	if err := json.Unmarshal(data, &aux); err != nil {
		return err
	}

	m.Language = aux.Language
	m.Date = aux.Date
	m.Subject = aux.Subject
	m.FormatType = aux.FormatType
	m.ImagePreprocessing = aux.ImagePreprocessing
	m.JSONSchema = aux.JSONSchema
	m.Error = aux.Error

	if err := m.decodeFormat(data); err != nil {
		return err
	}

	raw := map[string]json.RawMessage{}
	if err := json.Unmarshal(data, &raw); err == nil {
		m.Additional = make(map[string]json.RawMessage)
		for key, value := range raw {
			if _, ok := metadataCoreKeys[key]; ok {
				continue
			}
			if m.isFormatField(key) {
				continue
			}
			m.Additional[key] = value
		}
		if len(m.Additional) == 0 {
			m.Additional = nil
		}
	}

	return nil
}

func (m *Metadata) decodeFormat(data []byte) error {
	switch m.FormatType {
	case "pdf":
		var meta PdfMetadata
		if err := json.Unmarshal(data, &meta); err == nil {
			m.Pdf = &meta
		} else {
			return err
		}
	case "excel":
		var meta ExcelMetadata
		if err := json.Unmarshal(data, &meta); err == nil {
			m.Excel = &meta
		} else {
			return err
		}
	case "email":
		var meta EmailMetadata
		if err := json.Unmarshal(data, &meta); err == nil {
			m.Email = &meta
		} else {
			return err
		}
	case "pptx":
		var meta PptxMetadata
		if err := json.Unmarshal(data, &meta); err == nil {
			m.Pptx = &meta
		} else {
			return err
		}
	case "archive":
		var meta ArchiveMetadata
		if err := json.Unmarshal(data, &meta); err == nil {
			m.Archive = &meta
		} else {
			return err
		}
	case "image":
		var meta ImageMetadata
		if err := json.Unmarshal(data, &meta); err == nil {
			m.Image = &meta
		} else {
			return err
		}
	case "xml":
		var meta XMLMetadata
		if err := json.Unmarshal(data, &meta); err == nil {
			m.XML = &meta
		} else {
			return err
		}
	case "text":
		var meta TextMetadata
		if err := json.Unmarshal(data, &meta); err == nil {
			m.Text = &meta
		} else {
			return err
		}
	case "html":
		var meta HtmlMetadata
		if err := json.Unmarshal(data, &meta); err == nil {
			m.HTML = &meta
		} else {
			return err
		}
	case "ocr":
		var meta OcrMetadata
		if err := json.Unmarshal(data, &meta); err == nil {
			m.OCR = &meta
		} else {
			return err
		}
	}
	return nil
}

func (m *Metadata) isFormatField(key string) bool {
	if m.FormatType == "" {
		return false
	}
	if fields, ok := formatFieldSets[m.FormatType]; ok {
		for _, candidate := range fields {
			if candidate == key {
				return true
			}
		}
	}
	return false
}
