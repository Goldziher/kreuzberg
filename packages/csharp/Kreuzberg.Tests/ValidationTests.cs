using Xunit;

namespace Kreuzberg.Tests;

public class ValidationTests
{
    [Fact]
    public void ExtractFileSync_EmptyPath_Throws()
    {
        Assert.Throws<ArgumentException>(() => KreuzbergClient.ExtractFileSync(string.Empty));
    }

    [Fact]
    public void ExtractBytesSync_EmptyData_Throws()
    {
        Assert.Throws<KreuzbergValidationException>(() => KreuzbergClient.ExtractBytesSync(Array.Empty<byte>(), "application/pdf"));
    }

    [Fact]
    public void ExtractBytesSync_EmptyMime_Throws()
    {
        Assert.Throws<KreuzbergValidationException>(() => KreuzbergClient.ExtractBytesSync(new byte[] { 1, 2, 3 }, ""));
    }
}
