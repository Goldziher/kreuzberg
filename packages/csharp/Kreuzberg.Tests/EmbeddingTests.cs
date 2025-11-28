using System;
using System.Collections.Generic;
using Kreuzberg;
using Xunit;

namespace Kreuzberg.Tests;

public class EmbeddingTests
{
    public EmbeddingTests()
    {
        NativeTestHelper.EnsureNativeLibraryLoaded();
    }

    [Fact]
    public void ListEmbeddingPresets_ReturnsNonEmptyList()
    {
        var presets = KreuzbergClient.ListEmbeddingPresets();
        Assert.NotNull(presets);
        Assert.NotEmpty(presets);
    }

    [Fact]
    public void GetEmbeddingPreset_WithValidName_ReturnPreset()
    {
        // Get list of available presets
        var presets = KreuzbergClient.ListEmbeddingPresets();
        Assert.NotEmpty(presets);

        // Use the first available preset
        var presetName = presets[0];
        var preset = KreuzbergClient.GetEmbeddingPreset(presetName);

        Assert.NotNull(preset);
        Assert.Equal(presetName, preset.Name);
        Assert.True(preset.ChunkSize > 0);
        Assert.True(preset.Overlap >= 0);
        Assert.False(string.IsNullOrEmpty(preset.ModelName));
        Assert.True(preset.Dimensions > 0);
        Assert.False(string.IsNullOrEmpty(preset.Description));
    }

    [Fact]
    public void GetEmbeddingPreset_WithInvalidName_ReturnsNull()
    {
        var preset = KreuzbergClient.GetEmbeddingPreset("nonexistent-preset");
        Assert.Null(preset);
    }

    [Fact]
    public void GetEmbeddingPreset_WithEmptyName_ThrowsValidationException()
    {
        var ex = Assert.Throws<KreuzbergValidationException>(() => KreuzbergClient.GetEmbeddingPreset(""));
        Assert.Contains("preset name cannot be empty", ex.Message);
    }

    [Fact]
    public void GetEmbeddingPreset_WithNullName_ThrowsValidationException()
    {
        var ex = Assert.Throws<KreuzbergValidationException>(() => KreuzbergClient.GetEmbeddingPreset(null!));
        Assert.Contains("preset name cannot be empty", ex.Message);
    }

    [Fact]
    public void GetEmbeddingPreset_WithWhitespaceName_ThrowsValidationException()
    {
        var ex = Assert.Throws<KreuzbergValidationException>(() => KreuzbergClient.GetEmbeddingPreset("   "));
        Assert.Contains("preset name cannot be empty", ex.Message);
    }
}
