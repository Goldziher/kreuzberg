```csharp title="C#"
using System.Collections.Generic;
using System.Threading;

public class StatefulPlugin
{
    private readonly object _lock = new();
    private int _callCount = 0;
    private readonly Dictionary<string, object> _cache = new();

    public string Name() => "stateful-plugin";
    public string Version() => "1.0.0";

    public Dictionary<string, object> Process(Dictionary<string, object> result)
    {
        lock (_lock)
        {
            _callCount++;
            _cache["last_mime"] = result["mime_type"];
        }
        return result;
    }

    public void Initialize() { }
    public void Shutdown() { }
}
```
