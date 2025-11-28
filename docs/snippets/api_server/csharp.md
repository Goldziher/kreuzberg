```csharp
using System;
using System.Diagnostics;

class ApiServer
{
    static void Main()
    {
        var processInfo = new ProcessStartInfo
        {
            FileName = "kreuzberg",
            Arguments = "serve -H 0.0.0.0 -p 8000",
            UseShellExecute = false,
            RedirectStandardOutput = true,
            RedirectStandardError = true
        };

        using (var process = Process.Start(processInfo))
        {
            process?.WaitForExit();
        }
    }
}
```
