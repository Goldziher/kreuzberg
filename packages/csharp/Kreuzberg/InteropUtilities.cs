using System.Runtime.InteropServices;
using System.Text;

namespace Kreuzberg;

internal static class InteropUtilities
{
    internal static unsafe IntPtr AllocUtf8(string value)
    {
        var bytes = Encoding.UTF8.GetBytes(value);
        var size = (nuint)(bytes.Length + 1);
        var buffer = (byte*)NativeMemory.Alloc(size);
        var span = new Span<byte>(buffer, bytes.Length);
        bytes.AsSpan().CopyTo(span);
        buffer[bytes.Length] = 0;
        return (IntPtr)buffer;
    }

    internal static unsafe void FreeUtf8(IntPtr ptr)
    {
        if (ptr != IntPtr.Zero)
        {
            NativeMemory.Free((void*)ptr);
        }
    }

    internal static string? ReadUtf8(IntPtr ptr)
    {
        return ptr == IntPtr.Zero ? null : Marshal.PtrToStringUTF8(ptr);
    }

    internal static unsafe IntPtr[] ReadPointerArray(IntPtr ptr, int count)
    {
        var result = new IntPtr[count];
        var span = new ReadOnlySpan<IntPtr>((void*)ptr, count);
        span.CopyTo(result);
        return result;
    }
}
