using System.Buffers.Binary;
using System.Text;

namespace XRay.Content.Tests;

/// <summary>
/// Writes the smallest PSD files that exercise the parts of the format <c>PsdExtractor</c> reads: a
/// layer tree with groups, hidden layers and type layers, and image resources.
/// </summary>
/// <remarks>
/// A trimmed relative of Patchy's <c>tests/XRay.Psd.Tests/Support/PsdBuilder.cs</c> (MIT), the
/// test writer of the library X-Ray.Psd is built from. Everything pixel-related is gone: every
/// layer is 0x0 with no channels and the merged image is one raw 8-bit RGB pixel, because
/// extraction never decodes either. A type layer carries the minimum <c>TySh</c> the reader
/// accepts — a transform and a text descriptor whose only item is <c>Txt </c> — with no
/// EngineData, so it has text but no style runs or fonts.
/// </remarks>
internal sealed class SyntheticPsd
{
    /// <summary>One layer, or a group whose <see cref="Children"/> are listed top to bottom.</summary>
    public sealed class Layer
    {
        public string Name { get; init; } = "Layer";
        public string? Text { get; init; }
        public bool Hidden { get; init; }
        public List<Layer>? Children { get; init; }
    }

    /// <summary>Top-level layers, top to bottom (the order the Layers panel shows).</summary>
    public List<Layer> Layers { get; } = new();

    /// <summary>Image resources (id, data).</summary>
    public List<(ushort Id, byte[] Data)> Resources { get; } = new();

    /// <summary>Unicode alpha channel names (resource 1045) — a resource the reader types.</summary>
    public SyntheticPsd WithChannelNames(params string[] names)
    {
        var w = new Writer();
        foreach (var name in names) w.Unicode(name);
        Resources.Add((1045, w.ToArray()));
        return this;
    }

    public byte[] Build()
    {
        var output = new Writer();
        output.Ascii("8BPS");
        output.U16(1);
        output.Zeros(6);
        output.U16(3); // channels
        output.U32(1); // height
        output.U32(1); // width
        output.U16(8);
        output.U16(3); // RGB
        output.U32(0); // color mode data

        var resources = new Writer();
        foreach (var (id, data) in Resources)
        {
            resources.Ascii("8BIM");
            resources.U16(id);
            resources.U16(0); // empty Pascal name, padded to even
            resources.U32((uint)data.Length);
            resources.Bytes(data);
            if (data.Length % 2 != 0) resources.U8(0);
        }
        output.U32((uint)resources.Length);
        output.Bytes(resources.ToArray());

        // Records are stored bottom to top; a group is its closing divider, its children, then
        // the folder record that carries its name.
        var records = new List<(string Name, int Section, bool Hidden, string? Text)>();
        void Flatten(IReadOnlyList<Layer> layers)
        {
            for (int i = layers.Count - 1; i >= 0; i--)
            {
                var layer = layers[i];
                if (layer.Children is { } children)
                {
                    records.Add(("</Layer group>", 3, false, null));
                    Flatten(children);
                    records.Add((layer.Name, 1, layer.Hidden, null));
                }
                else
                {
                    records.Add((layer.Name, 0, layer.Hidden, layer.Text));
                }
            }
        }
        Flatten(Layers);

        var info = new Writer();
        info.I16((short)records.Count);
        foreach (var (name, section, hidden, text) in records)
        {
            info.Zeros(16); // empty rect
            info.U16(0); // no channels
            info.Ascii("8BIM");
            info.Ascii("norm");
            info.U8(255);
            info.U8(0);
            info.U8((byte)(hidden ? 0x02 : 0x00));
            info.U8(0);

            var extra = new Writer();
            extra.U32(0); // mask
            extra.U32(0); // blending ranges
            extra.Pascal(name, 4);
            var unicode = new Writer();
            unicode.Unicode(name);
            Block(extra, "luni", unicode.ToArray());
            if (section != 0)
            {
                var lsct = new Writer();
                lsct.U32((uint)section);
                Block(extra, "lsct", lsct.ToArray());
            }
            if (text is not null) Block(extra, "TySh", TypeTool(text));

            info.U32((uint)extra.Length);
            info.Bytes(extra.ToArray());
        }
        if (info.Length % 2 != 0) info.U8(0);

        var layerSection = new Writer();
        layerSection.U32((uint)info.Length);
        layerSection.Bytes(info.ToArray());
        layerSection.U32(0); // global layer mask info
        output.U32((uint)layerSection.Length);
        output.Bytes(layerSection.ToArray());

        output.U16(0); // raw merged image
        output.Bytes(new byte[] { 255, 255, 255 });
        return output.ToArray();
    }

    /// <summary>A <c>TySh</c> payload: version, identity transform, text version, text descriptor.</summary>
    private static byte[] TypeTool(string text)
    {
        var w = new Writer();
        w.U16(1);
        foreach (double v in new double[] { 1, 0, 0, 1, 0, 0 }) w.F64(v);
        w.U16(50);
        w.U32(16); // descriptor version
        w.Unicode(""); // class name
        w.U32(0);
        w.Ascii("TxLr"); // class id
        w.U32(1); // one item
        w.U32(0);
        w.Ascii("Txt ");
        w.Ascii("TEXT");
        // Photoshop separates paragraphs with \r and terminates the string with a NUL.
        w.Unicode(text.Replace('\n', '\r') + "\0");
        return w.ToArray();
    }

    private static void Block(Writer writer, string key, byte[] data)
    {
        writer.Ascii("8BIM");
        writer.Ascii(key);
        int padded = data.Length + data.Length % 2;
        writer.U32((uint)padded);
        writer.Bytes(data);
        writer.Zeros(padded - data.Length);
    }

    private sealed class Writer
    {
        private readonly MemoryStream _stream = new();

        public long Length => _stream.Length;
        public byte[] ToArray() => _stream.ToArray();
        public void U8(byte value) => _stream.WriteByte(value);
        public void Bytes(ReadOnlySpan<byte> data) => _stream.Write(data);
        public void Zeros(int count) => _stream.Write(new byte[count]);
        public void Ascii(string text) => _stream.Write(Encoding.ASCII.GetBytes(text));

        public void U16(ushort value)
        {
            Span<byte> b = stackalloc byte[2];
            BinaryPrimitives.WriteUInt16BigEndian(b, value);
            _stream.Write(b);
        }

        public void I16(short value) => U16(unchecked((ushort)value));

        public void U32(uint value)
        {
            Span<byte> b = stackalloc byte[4];
            BinaryPrimitives.WriteUInt32BigEndian(b, value);
            _stream.Write(b);
        }

        public void F64(double value)
        {
            Span<byte> b = stackalloc byte[8];
            BinaryPrimitives.WriteDoubleBigEndian(b, value);
            _stream.Write(b);
        }

        public void Pascal(string text, int padding)
        {
            var bytes = Encoding.Latin1.GetBytes(text);
            U8((byte)bytes.Length);
            Bytes(bytes);
            int consumed = bytes.Length + 1;
            Zeros((consumed + padding - 1) / padding * padding - consumed);
        }

        public void Unicode(string text)
        {
            U32((uint)text.Length);
            Bytes(Encoding.BigEndianUnicode.GetBytes(text));
        }
    }
}
