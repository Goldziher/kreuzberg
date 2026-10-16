using System.Text.Json.Serialization;

namespace XRay.Content.Types;

/// <summary>One interactive form field of a PDF (AcroForm), terminal fields only.</summary>
public sealed class PdfFormField
{
    /// <summary>The fully qualified name, partial names joined by dots.</summary>
    public string Name { get; set; } = "";

    /// <summary><c>text</c>, <c>checkbox</c>, <c>radio</c>, <c>choice</c>, <c>push_button</c>, <c>signature</c> or <c>unknown</c>.</summary>
    public string FieldType { get; set; } = "unknown";

    /// <summary>
    /// What a reader sees as the value: the text of a text field, <c>true</c>/<c>false</c> for a
    /// checkbox, the selected export value of a radio group, the selected option's display text
    /// for a choice field (several joined by <c>, </c>).
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public string? Value { get; set; }

    /// <summary>The raw values stored in <c>/V</c>: export values for a choice field or radio group.</summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public List<string>? Values { get; set; }

    /// <summary>For a checkbox: whether it is on.</summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public bool? Checked { get; set; }

    /// <summary>The choices of a choice field or radio group.</summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public List<PdfFormOption>? Options { get; set; }

    /// <summary>The field's <c>/DV</c>, as text.</summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public string? DefaultValue { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public uint? PageNumber { get; set; }

    /// <summary>The first widget's rectangle in default user space.</summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public BoundingBox? BoundingBox { get; set; }

    /// <summary>The field flags (<c>/Ff</c>), inherited through the parent chain.</summary>
    public uint Flags { get; set; }

    public bool ReadOnly { get; set; }
    public bool Required { get; set; }

    /// <summary>For a choice field: a drop-down (combo box) rather than a list box.</summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public bool? IsCombo { get; set; }

    /// <summary>For a choice field: more than one option may be selected.</summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public bool? MultiSelect { get; set; }
}

/// <summary>One option of a choice field or radio group.</summary>
public sealed class PdfFormOption
{
    /// <summary>The value stored in <c>/V</c> when the option is selected.</summary>
    public string Export { get; set; } = "";

    /// <summary>The text shown to the reader.</summary>
    public string Display { get; set; } = "";
}
