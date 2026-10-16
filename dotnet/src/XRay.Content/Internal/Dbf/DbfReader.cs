using System.Globalization;
using System.Text;

namespace XRay.Content.Internal.Dbf;

/// <summary>
/// Minimal dBASE (.dbf) reader. Mirrors the subset of the Rust `dbase` crate used by the DBF
/// extractor: field descriptors, field-type names, and record values as strings.
/// </summary>
internal sealed class DbfParsed
{
    public List<string> FieldNames { get; init; } = new();
    public List<string> FieldTypes { get; init; } = new();
    public List<List<string>> Rows { get; init; } = new();
    public int RecordCount { get; init; }
}

internal static class DbfReader
{
    private sealed class FieldDesc
    {
        public string Name = "";
        public char Type;
        public int Length;
        public int Decimal;
    }

    public static DbfParsed Parse(ReadOnlySpan<byte> content)
    {
        if (content.Length < 32) throw new InvalidDataException("Failed to open dBASE file: too short");
        byte[] data = content.ToArray();

        int numRecords = BitConverter.ToInt32(data, 4);
        int headerSize = BitConverter.ToUInt16(data, 8);
        int recordSize = BitConverter.ToUInt16(data, 10);

        var fields = new List<FieldDesc>();
        int off = 32;
        while (off < data.Length && data[off] != 0x0D)
        {
            int nameEnd = off;
            while (nameEnd < off + 11 && data[nameEnd] != 0) nameEnd++;
            string name = Encoding.ASCII.GetString(data, off, nameEnd - off);
            char type = (char)data[off + 11];
            int length = data[off + 16];
            int dec = data[off + 17];
            fields.Add(new FieldDesc { Name = name, Type = type, Length = length, Decimal = dec });
            off += 32;
        }

        var fieldNames = fields.Select(f => f.Name).ToList();
        var fieldTypes = Enumerable.Repeat("Unknown", fields.Count).ToList();
        var rows = new List<List<string>>();
        bool firstRow = true;

        int pos = headerSize;
        for (int r = 0; r < numRecords && pos + recordSize <= data.Length; r++, pos += recordSize)
        {
            Cancel.Every(r, 0x3FF);
            byte deletionFlag = data[pos];
            if (deletionFlag == 0x2A) continue; // deleted record
            var row = new List<string>(fields.Count);
            int fieldPos = pos + 1;
            for (int c = 0; c < fields.Count; c++)
            {
                var f = fields[c];
                string raw = DecodeField(data, fieldPos, f.Length);
                if (firstRow) fieldTypes[c] = FieldTypeName(f.Type);
                row.Add(FieldValueToString(f.Type, raw));
                fieldPos += f.Length;
            }
            rows.Add(row);
            firstRow = false;
        }

        return new DbfParsed
        {
            FieldNames = fieldNames,
            FieldTypes = fieldTypes,
            Rows = rows,
            RecordCount = rows.Count,
        };
    }

    private static string DecodeField(byte[] data, int start, int len)
    {
        int end = Math.Min(start + len, data.Length);
        return Encoding.UTF8.GetString(data, start, end - start);
    }

    private static string FieldTypeName(char type) => type switch
    {
        'C' => "Character",
        'N' => "Numeric",
        'L' => "Logical",
        'D' => "Date",
        'F' => "Float",
        'I' => "Integer",
        'Y' => "Currency",
        'B' or 'O' => "Double",
        'M' => "Memo",
        _ => "Unknown",
    };

    private static string FieldValueToString(char type, string raw)
    {
        switch (type)
        {
            case 'C':
            case 'M':
                return raw.Trim();
            case 'N':
            case 'F':
            {
                string t = raw.Trim();
                if (t.Length == 0) return "";
                return double.TryParse(t, NumberStyles.Any, CultureInfo.InvariantCulture, out var n)
                    ? n.ToString(CultureInfo.InvariantCulture) : "";
            }
            case 'L':
            {
                string t = raw.Trim();
                if (t.Length == 0) return "";
                char c = char.ToUpperInvariant(t[0]);
                if (c is 'T' or 'Y') return "true";
                if (c is 'F' or 'N') return "false";
                return "";
            }
            case 'I':
            {
                string t = raw.Trim();
                return long.TryParse(t, NumberStyles.Any, CultureInfo.InvariantCulture, out var i) ? i.ToString(CultureInfo.InvariantCulture) : "";
            }
            case 'B':
            case 'O':
            {
                string t = raw.Trim();
                return double.TryParse(t, NumberStyles.Any, CultureInfo.InvariantCulture, out var d) ? d.ToString(CultureInfo.InvariantCulture) : "";
            }
            default:
                return "";
        }
    }
}
