using Kreuzberg;
using System.Collections.Generic;
using System.Security.Cryptography;
using System.Text;

// NOTE: ICacheBackend interface is not available in C# bindings
// This example shows a custom caching pattern using a wrapper

class CustomCacheWrapper
{
    private readonly Dictionary<string, (ExtractionResult result, DateTime timestamp)> _cache;
    private readonly TimeSpan _cacheExpiration;

    public CustomCacheWrapper(TimeSpan? cacheExpiration = null)
    {
        _cache = new Dictionary<string, (ExtractionResult, DateTime)>();
        _cacheExpiration = cacheExpiration ?? TimeSpan.FromHours(1);
    }

    public ExtractionResult? Get(string key)
    {
        if (_cache.TryGetValue(key, out var entry))
        {
            if (DateTime.UtcNow - entry.timestamp < _cacheExpiration)
            {
                return entry.result;
            }
            else
            {
                _cache.Remove(key);
            }
        }

        return null;
    }

    public void Set(string key, ExtractionResult result)
    {
        _cache[key] = (result, DateTime.UtcNow);
    }

    public void Delete(string key)
    {
        _cache.Remove(key);
    }

    public void Clear()
    {
        _cache.Clear();
    }

    public string GenerateKey(string filePath, ExtractionConfig? config)
    {
        var keyData = $"{filePath}:{config?.GetHashCode() ?? 0}";
        using var sha256 = SHA256.Create();
        var hashBytes = sha256.ComputeHash(Encoding.UTF8.GetBytes(keyData));
        return Convert.ToHexString(hashBytes);
    }

    public ExtractionResult GetOrExtract(string filePath, ExtractionConfig? config = null)
    {
        var cacheKey = GenerateKey(filePath, config);

        // Check cache first
        var cached = Get(cacheKey);
        if (cached != null)
        {
            Console.WriteLine("Retrieved from cache");
            return cached;
        }

        // Extract and cache
        var result = KreuzbergClient.ExtractFileSync(filePath, config);
        Set(cacheKey, result);
        Console.WriteLine("Extracted and cached");

        return result;
    }
}

class Program
{
    static void Main()
    {
        var cache = new CustomCacheWrapper(cacheExpiration: TimeSpan.FromMinutes(30));

        try
        {
            var config = new ExtractionConfig { UseCache = true };
            var filePath = "document.pdf";

            // First extraction - will be cached
            var result1 = cache.GetOrExtract(filePath, config);
            Console.WriteLine($"First extraction: {result1.Content.Length} chars");

            // Second extraction - retrieved from cache
            var result2 = cache.GetOrExtract(filePath, config);
            Console.WriteLine($"Second extraction: {result2.Content.Length} chars");

            // Clear cache
            cache.Clear();
            Console.WriteLine("Cache cleared");
        }
        catch (KreuzbergException ex)
        {
            Console.WriteLine($"Error: {ex.Message}");
        }
    }
}
