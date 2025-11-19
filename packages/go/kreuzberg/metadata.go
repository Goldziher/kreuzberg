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

var formatFieldSets = map[FormatType][]string{
	FormatPDF: {
		"title", "subject", "authors", "keywords", "created_at", "modified_at",
		"created_by", "producer", "page_count", "pdf_version", "is_encrypted",
		"width", "height", "summary",
	},
	FormatExcel:   {"sheet_count", "sheet_names"},
	FormatEmail:   {"from_email", "from_name", "to_emails", "cc_emails", "bcc_emails", "message_id", "attachments"},
	FormatPPTX:    {"title", "author", "description", "summary", "fonts"},
	FormatArchive: {"format", "file_count", "file_list", "total_size", "compressed_size"},
	FormatImage:   {"width", "height", "format", "exif"},
	FormatXML:     {"element_count", "unique_elements"},
	FormatText:    {"line_count", "word_count", "character_count", "headers", "links", "code_blocks"},
	FormatHTML: {
		"title", "description", "keywords", "author", "canonical", "base_href",
		"og_title", "og_description", "og_image", "og_url", "og_type", "og_site_name",
		"twitter_card", "twitter_title", "twitter_description", "twitter_image", "twitter_site", "twitter_creator",
		"link_author", "link_license", "link_alternate",
	},
	FormatOCR: {"language", "psm", "output_format", "table_count", "table_rows", "table_cols"},
}

// UnmarshalJSON ensures Metadata captures flattened format unions and additional custom fields.
func (m *Metadata) UnmarshalJSON(data []byte) error {
	raw := map[string]json.RawMessage{}
	if err := json.Unmarshal(data, &raw); err != nil {
		return err
	}

	decodeString := func(key string) *string {
		value, exists := raw[key]
		if !exists {
			return nil
		}
		var out string
		if err := json.Unmarshal(value, &out); err != nil {
			return nil
		}
		return &out
	}

	m.Language = decodeString("language")
	m.Date = decodeString("date")
	m.Subject = decodeString("subject")

	if value, ok := raw["image_preprocessing"]; ok {
		var meta ImagePreprocessingMetadata
		if err := json.Unmarshal(value, &meta); err == nil {
			m.ImagePreprocessing = &meta
		}
	}
	if value, ok := raw["json_schema"]; ok {
		m.JSONSchema = value
	}
	if value, ok := raw["error"]; ok {
		var errMeta ErrorMetadata
		if err := json.Unmarshal(value, &errMeta); err == nil {
			m.Error = &errMeta
		}
	}
	if value, ok := raw["format_type"]; ok {
		var format string
		if err := json.Unmarshal(value, &format); err == nil {
			m.Format.Type = FormatType(format)
		}
	}

	if err := m.decodeFormat(data); err != nil {
		return err
	}

	recognized := map[string]struct{}{}
	for key := range metadataCoreKeys {
		recognized[key] = struct{}{}
	}
	for _, field := range formatFieldSets[m.Format.Type] {
		recognized[field] = struct{}{}
	}

	m.Additional = make(map[string]json.RawMessage)
	for key, value := range raw {
		if _, ok := recognized[key]; ok {
			continue
		}
		m.Additional[key] = value
	}
	if len(m.Additional) == 0 {
		m.Additional = nil
	}

	return nil
}

// MarshalJSON reserializes Metadata back into the flattened JSON structure that
// the Rust core produces so round-tripping preserves the original payload.
func (m Metadata) MarshalJSON() ([]byte, error) {
	out := make(map[string]any)

	if m.Language != nil {
		out["language"] = *m.Language
	}
	if m.Date != nil {
		out["date"] = *m.Date
	}
	if m.Subject != nil {
		out["subject"] = *m.Subject
	}
	if m.ImagePreprocessing != nil {
		out["image_preprocessing"] = m.ImagePreprocessing
	}
	if m.JSONSchema != nil {
		out["json_schema"] = json.RawMessage(m.JSONSchema)
	}
	if m.Error != nil {
		out["error"] = m.Error
	}

	formatFields, err := m.encodeFormat()
	if err != nil {
		return nil, err
	}
	for key, value := range formatFields {
		out[key] = value
	}

	for key, value := range m.Additional {
		out[key] = json.RawMessage(value)
	}

	return json.Marshal(out)
}

func (m *Metadata) decodeFormat(data []byte) error {
	switch m.Format.Type {
	case FormatPDF:
		var meta PdfMetadata
		if err := json.Unmarshal(data, &meta); err != nil {
			return err
		}
		m.Format.Pdf = &meta
	case FormatExcel:
		var meta ExcelMetadata
		if err := json.Unmarshal(data, &meta); err != nil {
			return err
		}
		m.Format.Excel = &meta
	case FormatEmail:
		var meta EmailMetadata
		if err := json.Unmarshal(data, &meta); err != nil {
			return err
		}
		m.Format.Email = &meta
	case FormatPPTX:
		var meta PptxMetadata
		if err := json.Unmarshal(data, &meta); err != nil {
			return err
		}
		m.Format.Pptx = &meta
	case FormatArchive:
		var meta ArchiveMetadata
		if err := json.Unmarshal(data, &meta); err != nil {
			return err
		}
		m.Format.Archive = &meta
	case FormatImage:
		var meta ImageMetadata
		if err := json.Unmarshal(data, &meta); err != nil {
			return err
		}
		m.Format.Image = &meta
	case FormatXML:
		var meta XMLMetadata
		if err := json.Unmarshal(data, &meta); err != nil {
			return err
		}
		m.Format.XML = &meta
	case FormatText:
		var meta TextMetadata
		if err := json.Unmarshal(data, &meta); err != nil {
			return err
		}
		m.Format.Text = &meta
	case FormatHTML:
		var meta HtmlMetadata
		if err := json.Unmarshal(data, &meta); err != nil {
			return err
		}
		m.Format.HTML = &meta
	case FormatOCR:
		var meta OcrMetadata
		if err := json.Unmarshal(data, &meta); err != nil {
			return err
		}
		m.Format.OCR = &meta
	default:
		m.Format.Type = FormatUnknown
	}
	return nil
}

func (m Metadata) encodeFormat() (map[string]json.RawMessage, error) {
	result := make(map[string]json.RawMessage)
	if m.Format.Type == FormatUnknown || m.Format.Type == "" {
		return result, nil
	}

	typeRaw, err := json.Marshal(m.Format.Type)
	if err != nil {
		return nil, err
	}
	result["format_type"] = json.RawMessage(typeRaw)

	var payload any
	switch m.Format.Type {
	case FormatPDF:
		payload = m.Format.Pdf
	case FormatExcel:
		payload = m.Format.Excel
	case FormatEmail:
		payload = m.Format.Email
	case FormatPPTX:
		payload = m.Format.Pptx
	case FormatArchive:
		payload = m.Format.Archive
	case FormatImage:
		payload = m.Format.Image
	case FormatXML:
		payload = m.Format.XML
	case FormatText:
		payload = m.Format.Text
	case FormatHTML:
		payload = m.Format.HTML
	case FormatOCR:
		payload = m.Format.OCR
	}

	if payload == nil {
		return result, nil
	}

	fields, err := encodeStructToRaw(payload)
	if err != nil {
		return nil, err
	}
	for key, value := range fields {
		result[key] = value
	}
	return result, nil
}

func encodeStructToRaw(value any) (map[string]json.RawMessage, error) {
	raw, err := json.Marshal(value)
	if err != nil {
		return nil, err
	}
	result := make(map[string]json.RawMessage)
	if err := json.Unmarshal(raw, &result); err != nil {
		return nil, err
	}
	return result, nil
}
