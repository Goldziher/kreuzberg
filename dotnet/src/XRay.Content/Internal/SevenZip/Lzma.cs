// Managed LZMA / LZMA2 decoder — ports the canonical decoder from the LZMA specification
// (LzmaSpec.cpp reference implementation), as used by the `sevenz-rust2` crate behind Rust's
// 7z extractor. Decodes into a fully-materialised output buffer of known size (7z folders
// always carry their unpack size), so the dictionary window IS the output buffer.

namespace XRay.Content.Internal.SevenZip;

internal sealed class LzmaException : Exception
{
    public LzmaException(string message) : base(message) { }
}

/// <summary>LZMA decoder state: probability models + machine state, reusable across LZMA2 chunks.</summary>
internal sealed class LzmaDecoder
{
    private const int NumStates = 12;
    private const ushort ProbInit = 1024; // 2048 / 2

    private int _lc, _lp, _pb;
    private int _pbMask, _lpMask;
    private int _tick;

    // Probability models.
    private ushort[] _isMatch = null!;      // [state << 4 | posState]
    private ushort[] _isRep = null!, _isRepG0 = null!, _isRepG1 = null!, _isRepG2 = null!;
    private ushort[] _isRep0Long = null!;   // [state << 4 | posState]
    private ushort[] _posSlot = null!;      // 4 trees × 64
    private ushort[] _specPos = null!;      // 128 (spec: 115 used)
    private ushort[] _align = null!;        // 16
    private ushort[] _lenChoice = null!, _lenLow = null!, _lenMid = null!, _lenHigh = null!;       // match len
    private ushort[] _repChoice = null!, _repLow = null!, _repMid = null!, _repHigh = null!;       // rep len
    private ushort[] _literals = null!;     // 0x300 << (lc + lp)

    // Machine state.
    private int _state;
    private uint _rep0, _rep1, _rep2, _rep3;

    // Range decoder.
    private ReadOnlyMemory<byte> _input;
    private int _inPos;
    private uint _range, _code;

    // Output (dictionary window == output buffer).
    private readonly byte[] _output;
    private int _outPos;

    public int OutPos => _outPos;

    public LzmaDecoder(byte[] output)
    {
        _output = output;
        SetProps(3, 0, 2);
        ResetState();
    }

    /// <summary>Set lc/lp/pb from the packed properties byte (props = (pb*5 + lp)*9 + lc).</summary>
    public void SetPropsByte(byte d)
    {
        if (d >= 9 * 5 * 5) throw new LzmaException("invalid LZMA properties byte");
        int lc = d % 9; d /= 9;
        int lp = d % 5;
        int pb = d / 5;
        SetProps(lc, lp, pb);
    }

    public void SetProps(int lc, int lp, int pb)
    {
        _lc = lc; _lp = lp; _pb = pb;
        _pbMask = (1 << pb) - 1;
        _lpMask = (1 << lp) - 1;
        _literals = NewProbs(0x300 << (lc + lp));
    }

    /// <summary>Reset probability models, machine state and rep distances.</summary>
    public void ResetState()
    {
        _state = 0;
        _rep0 = _rep1 = _rep2 = _rep3 = 0;
        _isMatch = NewProbs(NumStates << 4);
        _isRep = NewProbs(NumStates);
        _isRepG0 = NewProbs(NumStates);
        _isRepG1 = NewProbs(NumStates);
        _isRepG2 = NewProbs(NumStates);
        _isRep0Long = NewProbs(NumStates << 4);
        _posSlot = NewProbs(4 * 64);
        _specPos = NewProbs(128);
        _align = NewProbs(16);
        _lenChoice = NewProbs(2);
        _lenLow = NewProbs(16 * 8);
        _lenMid = NewProbs(16 * 8);
        _lenHigh = NewProbs(256);
        _repChoice = NewProbs(2);
        _repLow = NewProbs(16 * 8);
        _repMid = NewProbs(16 * 8);
        _repHigh = NewProbs(256);
        Array.Fill(_literals, ProbInit);
    }

    private static ushort[] NewProbs(int n)
    {
        var p = new ushort[n];
        Array.Fill(p, ProbInit);
        return p;
    }

    /// <summary>Copy an uncompressed LZMA2 chunk straight into the window.</summary>
    public void CopyUncompressed(ReadOnlySpan<byte> chunk)
    {
        chunk.CopyTo(_output.AsSpan(_outPos));
        _outPos += chunk.Length;
    }

    /// <summary>
    /// Decode one range-coded chunk: produce exactly <paramref name="unpackLen"/> bytes.
    /// LZMA1 = a single chunk covering the whole stream.
    /// </summary>
    public void DecodeChunk(ReadOnlyMemory<byte> packed, int unpackLen)
    {
        _input = packed;
        _inPos = 0;
        RangeInit();

        int target = _outPos + unpackLen;
        if (target > _output.Length) throw new LzmaException("LZMA output overflow");

        while (_outPos < target)
        {
            Cancel.Every(_tick++, 0x3FFF);
            int posState = _outPos & _pbMask;

            if (DecodeBit(_isMatch, (_state << 4) | posState) == 0)
            {
                DecodeLiteral();
                _state = _state < 4 ? 0 : _state < 10 ? _state - 3 : _state - 6;
                continue;
            }

            int len;
            if (DecodeBit(_isRep, _state) != 0)
            {
                if (_outPos == 0) throw new LzmaException("rep match at stream start");
                if (DecodeBit(_isRepG0, _state) == 0)
                {
                    if (DecodeBit(_isRep0Long, (_state << 4) | posState) == 0)
                    {
                        // Short rep: single byte at distance rep0.
                        _state = _state < 7 ? 9 : 11;
                        _output[_outPos] = _output[_outPos - (int)_rep0 - 1];
                        _outPos++;
                        continue;
                    }
                }
                else
                {
                    uint dist;
                    if (DecodeBit(_isRepG1, _state) == 0) dist = _rep1;
                    else
                    {
                        if (DecodeBit(_isRepG2, _state) == 0) dist = _rep2;
                        else { dist = _rep3; _rep3 = _rep2; }
                        _rep2 = _rep1;
                    }
                    _rep1 = _rep0;
                    _rep0 = dist;
                }
                len = DecodeLen(_repChoice, _repLow, _repMid, _repHigh, posState);
                _state = _state < 7 ? 8 : 11;
            }
            else
            {
                _rep3 = _rep2; _rep2 = _rep1; _rep1 = _rep0;
                len = DecodeLen(_lenChoice, _lenLow, _lenMid, _lenHigh, posState);
                _state = _state < 7 ? 7 : 10;

                // Distance from the 0-based length.
                int lenState = len < 4 ? len : 3;
                int posSlot = BitTreeDecode(_posSlot, lenState * 64, 6);
                if (posSlot < 4)
                {
                    _rep0 = (uint)posSlot;
                }
                else
                {
                    int numDirect = (posSlot >> 1) - 1;
                    uint dist = (uint)(2 | (posSlot & 1)) << numDirect;
                    if (posSlot < 14)
                        dist += BitTreeReverseDecode(_specPos, (int)dist - posSlot, numDirect);
                    else
                    {
                        dist += DecodeDirectBits(numDirect - 4) << 4;
                        dist += BitTreeReverseDecode(_align, 0, 4);
                    }
                    _rep0 = dist;
                }
                if (_rep0 == 0xFFFFFFFF) break; // end-of-stream marker
            }

            int copyLen = len + 2; // kMatchMinLen
            int srcBase = _outPos - (int)_rep0 - 1;
            if (srcBase < 0) throw new LzmaException("LZMA match distance out of range");
            if (_outPos + copyLen > target) copyLen = target - _outPos; // clamp (spec allows exact fill)
            for (int i = 0; i < copyLen; i++)
            {
                _output[_outPos] = _output[srcBase + i];
                _outPos++;
            }
        }
    }

    private void DecodeLiteral()
    {
        uint prevByte = _outPos > 0 ? _output[_outPos - 1] : 0u;
        int litState = (int)((((uint)_outPos & (uint)_lpMask) << _lc) + (prevByte >> (8 - _lc)));
        int probsBase = 0x300 * litState;

        uint symbol = 1;
        if (_state >= 7)
        {
            if (_outPos - (int)_rep0 - 1 < 0) throw new LzmaException("literal matchByte out of range");
            uint matchByte = _output[_outPos - (int)_rep0 - 1];
            do
            {
                uint matchBit = (matchByte >> 7) & 1;
                matchByte <<= 1;
                uint bit = (uint)DecodeBit(_literals, probsBase + (int)(((1 + matchBit) << 8) + symbol));
                symbol = (symbol << 1) | bit;
                if (matchBit != bit)
                {
                    while (symbol < 0x100)
                        symbol = (symbol << 1) | (uint)DecodeBit(_literals, probsBase + (int)symbol);
                    break;
                }
            } while (symbol < 0x100);
        }
        else
        {
            while (symbol < 0x100)
                symbol = (symbol << 1) | (uint)DecodeBit(_literals, probsBase + (int)symbol);
        }
        _output[_outPos++] = (byte)symbol;
    }

    // Returns the 0-based length (actual = value + 2).
    private int DecodeLen(ushort[] choice, ushort[] low, ushort[] mid, ushort[] high, int posState)
    {
        if (DecodeBit(choice, 0) == 0)
            return BitTreeDecode(low, posState * 8, 3);
        if (DecodeBit(choice, 1) == 0)
            return 8 + BitTreeDecode(mid, posState * 8, 3);
        return 16 + BitTreeDecode(high, 0, 8);
    }

    // ── range decoder ───────────────────────────────────────────────────────────
    private void RangeInit()
    {
        if (_input.Length < 5) throw new LzmaException("LZMA chunk too short");
        var s = _input.Span;
        _inPos = 1; // first byte must be 0 and is skipped
        _code = ((uint)s[1] << 24) | ((uint)s[2] << 16) | ((uint)s[3] << 8) | s[4];
        _inPos = 5;
        _range = 0xFFFFFFFF;
    }

    private byte NextInputByte() => _inPos < _input.Length ? _input.Span[_inPos++] : (byte)0;

    private void Normalize()
    {
        if (_range < (1u << 24))
        {
            _range <<= 8;
            _code = (_code << 8) | NextInputByte();
        }
    }

    private int DecodeBit(ushort[] probs, int index)
    {
        uint prob = probs[index];
        uint bound = (_range >> 11) * prob;
        int bit;
        if (_code < bound)
        {
            _range = bound;
            probs[index] = (ushort)(prob + ((2048 - prob) >> 5));
            bit = 0;
        }
        else
        {
            _range -= bound;
            _code -= bound;
            probs[index] = (ushort)(prob - (prob >> 5));
            bit = 1;
        }
        Normalize();
        return bit;
    }

    private uint DecodeDirectBits(int count)
    {
        uint result = 0;
        for (int i = 0; i < count; i++)
        {
            _range >>= 1;
            _code -= _range;
            uint t = 0u - (_code >> 31);
            _code += _range & t;
            Normalize();
            result = (result << 1) + (t + 1);
        }
        return result;
    }

    private int BitTreeDecode(ushort[] probs, int offset, int numBits)
    {
        int m = 1;
        for (int i = 0; i < numBits; i++)
            m = (m << 1) | DecodeBit(probs, offset + m);
        return m - (1 << numBits);
    }

    private uint BitTreeReverseDecode(ushort[] probs, int offset, int numBits)
    {
        int m = 1;
        uint symbol = 0;
        for (int i = 0; i < numBits; i++)
        {
            int bit = DecodeBit(probs, offset + m);
            m = (m << 1) | bit;
            symbol |= (uint)bit << i;
        }
        return symbol;
    }
}

internal static class Lzma
{
    /// <summary>Decode a raw LZMA1 stream (no container header) into exactly unpackSize bytes.</summary>
    public static byte[] DecodeLzma1(byte propsByte, ReadOnlyMemory<byte> packed, long unpackSize)
    {
        var output = new byte[checked((int)unpackSize)];
        var dec = new LzmaDecoder(output);
        dec.SetPropsByte(propsByte);
        dec.ResetState();
        dec.DecodeChunk(packed, output.Length);
        if (dec.OutPos != output.Length) throw new LzmaException("LZMA stream ended early");
        return output;
    }

    /// <summary>Decode an LZMA2 stream (chunked LZMA with per-chunk reset control) into unpackSize bytes.</summary>
    public static byte[] DecodeLzma2(ReadOnlyMemory<byte> packed, long unpackSize)
    {
        var output = new byte[checked((int)unpackSize)];
        var dec = new LzmaDecoder(output);
        var span = packed;
        int pos = 0;

        while (pos < span.Length && dec.OutPos < output.Length)
        {
            Cancel.ThrowIfRequested();
            byte control = span.Span[pos++];
            if (control == 0) break; // end of stream

            if (control < 3)
            {
                // 1 = uncompressed chunk + dict reset, 2 = uncompressed chunk. 2-byte size-1.
                if (pos + 2 > span.Length) throw new LzmaException("LZMA2 truncated chunk header");
                int size = ((span.Span[pos] << 8) | span.Span[pos + 1]) + 1;
                pos += 2;
                if (pos + size > span.Length) throw new LzmaException("LZMA2 truncated uncompressed chunk");
                dec.CopyUncompressed(span.Span.Slice(pos, size));
                pos += size;
                dec.ResetState(); // an uncompressed chunk invalidates the machine state
            }
            else if (control >= 0x80)
            {
                if (pos + 4 > span.Length) throw new LzmaException("LZMA2 truncated chunk header");
                int unpackLen = ((control & 0x1F) << 16) + ((span.Span[pos] << 8) | span.Span[pos + 1]) + 1;
                int packLen = ((span.Span[pos + 2] << 8) | span.Span[pos + 3]) + 1;
                pos += 4;
                int mode = (control >> 5) & 3;
                if (mode >= 2)
                {
                    if (pos >= span.Length) throw new LzmaException("LZMA2 missing props byte");
                    dec.SetPropsByte(span.Span[pos++]);
                }
                if (mode >= 1) dec.ResetState();
                if (pos + packLen > span.Length) throw new LzmaException("LZMA2 truncated packed chunk");
                dec.DecodeChunk(span.Slice(pos, packLen), unpackLen);
                pos += packLen;
            }
            else
            {
                throw new LzmaException($"invalid LZMA2 control byte 0x{control:X2}");
            }
        }

        if (dec.OutPos != output.Length) throw new LzmaException("LZMA2 stream ended early");
        return output;
    }
}
