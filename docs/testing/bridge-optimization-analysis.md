# Bridge Optimization Analysis

## Executive Summary

Analysis of PyO3 (Python) and NAPI-RS (Node.js) FFI bridges to identify optimization opportunities. Both bridges show excellent performance characteristics, but there are potential optimizations to reduce overhead and memory usage.

## Current Performance Baselines

### PyO3 Bridge (Rust ↔ Python)
- **Time overhead**: ~30ms/iter
- **Memory leak**: 1.22 KB/iter (stable)
- **Extraction rate**: 34/s
- **Overall assessment**: Excellent (6% FFI overhead)

### NAPI-RS Bridge (Rust ↔ Node.js)
- **Time overhead**: ~45ms/iter
- **Short-term leak**: 67 KB/iter
- **Long-term leak**: 2.40 KB/iter (stabilizes)
- **Extraction rate**: 21.5/s
- **Overall assessment**: Good (typical for Node.js FFI)

## Potential Optimizations

### 1. **String Handling Optimizations**

#### Current Behavior
Both bridges convert Rust `String` → Python/JS `String` on every extraction:
- PyO3: Uses PyO3's automatic conversion
- NAPI-RS: Uses NAPI string conversion

#### Optimization Opportunities

**A. Intern Common Strings**
- Metadata keys are repeated: "mime_type", "metadata", "tables", etc.
- **Impact**: Reduce allocations for dictionary keys
- **Complexity**: Low
- **Expected gain**: 5-10% memory reduction

**B. Use String Views for Large Content**
- Currently: Full copy from Rust → Python/JS
- Proposed: Zero-copy string views where possible
- **Impact**: Reduce memory allocations
- **Complexity**: High (requires unsafe code)
- **Expected gain**: 10-20% memory reduction, 5% speed improvement
- **Risk**: Complex lifetime management, potential UB

### 2. **Metadata Serialization Optimization**

#### Current Behavior
```rust
// PyO3
metadata: HashMap<String, Value> → PyDict
```

```typescript
// NAPI-RS
metadata: JSON.parse(metadataStr)
```

#### Issues
- NAPI-RS: Double serialization (Rust → JSON string → JS object)
- PyO3: Multiple allocations for dictionary creation

#### Optimization Opportunities

**A. Direct Object Creation (NAPI-RS)**
- Remove JSON serialization step
- Create JS object directly from Rust HashMap
- **Impact**: Reduce allocations, improve speed
- **Complexity**: Medium
- **Expected gain**: 10-15% speed improvement for metadata-heavy documents

**B. Reuse Metadata Keys (PyO3)**
- Intern common metadata keys (author, title, etc.)
- **Impact**: Reduce string allocations
- **Complexity**: Low
- **Expected gain**: 5-10% memory reduction

### 3. **Result Type Optimization**

#### Current Behavior
Both bridges allocate new ExtractionResult objects on every call.

#### Optimization Opportunities

**A. Object Pooling (NAPI-RS)**
- Reuse JS objects for repeated extractions
- Clear and repopulate instead of allocate
- **Impact**: Reduce GC pressure
- **Complexity**: Medium
- **Expected gain**: 20-30% reduction in short-term memory spikes
- **Tradeoff**: More complex API, potential user-facing bugs

**B. Lazy Metadata/Tables (Both)**
- Don't populate metadata/tables unless accessed
- Use getters to compute on-demand
- **Impact**: Reduce allocations for simple use cases
- **Complexity**: High (API change)
- **Expected gain**: 30-50% speed improvement for content-only extraction
- **Risk**: Breaking change for existing users

### 4. **Batch Processing Optimization**

#### Current Behavior
```rust
// PyO3
for result in results {
    list.append(ExtractionResult::from_rust(result, py)?)?;
}
```

```typescript
// NAPI-RS (converted via index.ts)
rawResults.map(convertResult)
```

#### Optimization Opportunities

**A. Pre-allocate Result Arrays**
- Allocate array with known size upfront
- **Impact**: Reduce dynamic growth overhead
- **Complexity**: Low
- **Expected gain**: 5-10% batch processing improvement

**B. Parallel Conversion (NAPI-RS)**
- Convert Rust results to JS objects in parallel
- Use Worker threads for conversion
- **Impact**: Improve batch processing speed
- **Complexity**: High
- **Expected gain**: 20-40% batch processing improvement
- **Risk**: Complex synchronization, may not be worth it

### 5. **Config Object Optimization**

#### Current Behavior
Config is converted on every call:
```rust
let rust_config: kreuzberg::ExtractionConfig = config.into();
```

#### Optimization Opportunities

**A. Config Caching**
- Cache converted configs by hash
- Reuse if config hasn't changed
- **Impact**: Reduce allocations for repeated extractions with same config
- **Complexity**: Medium
- **Expected gain**: 10-20% improvement for repeated calls
- **Tradeoff**: Memory overhead for cache

**B. Default Config Singleton**
- Pre-allocate default config
- Reuse instead of creating on each call
- **Impact**: Reduce default case allocations
- **Complexity**: Low
- **Expected gain**: 5-10% improvement when using defaults

### 6. **GC Tuning (NAPI-RS Only)**

#### Current Issue
V8 GC is conservative, leading to 67 KB/iter short-term leaks.

#### Optimization Opportunities

**A. Explicit GC Hints**
- Call `global.gc()` after batch operations
- Provide API for users to trigger GC
- **Impact**: Reduce short-term memory spikes
- **Complexity**: Low
- **Expected gain**: Eliminate short-term spikes (67 KB → 2 KB)
- **Tradeoff**: Requires `--expose-gc` flag

**B. WeakRef for Large Objects**
- Use WeakRef for large content strings
- Allow GC to reclaim sooner
- **Impact**: Reduce memory pressure
- **Complexity**: High
- **Expected gain**: 20-30% memory reduction
- **Risk**: Complex API, may not work well

## Recommended Optimizations

### High Priority (Implement First)

1. **Default Config Singleton** (Low complexity, 5-10% gain)
   - Easy win, no API changes
   - Affects all users positively

2. **String Interning for Metadata Keys** (Low complexity, 5-10% gain)
   - Simple change, measurable impact
   - No API changes required

3. **Direct Metadata Object Creation (NAPI-RS)** (Medium complexity, 10-15% gain)
   - Removes double serialization
   - Significant improvement for metadata-heavy docs

### Medium Priority (Consider)

4. **Config Caching** (Medium complexity, 10-20% gain)
   - Helps repeated extractions
   - Some memory overhead acceptable

5. **Pre-allocate Result Arrays** (Low complexity, 5-10% batch gain)
   - Simple optimization
   - Affects batch processing only

### Low Priority (Evaluate Later)

6. **Lazy Metadata/Tables** (High complexity, API change)
   - Breaking change, requires major version
   - High gain but high risk

7. **Zero-copy String Views** (High complexity, unsafe)
   - Requires unsafe code
   - Complex lifetime management
   - Risk of UB outweighs benefits

### Not Recommended

8. **Object Pooling** (Medium complexity, complex API)
   - Adds API complexity
   - Potential user-facing bugs
   - Memory savings not worth tradeoff

9. **Parallel Conversion** (High complexity, synchronization issues)
   - Overhead likely exceeds benefits
   - Complex to maintain

## Implementation Plan

### Phase 1: Low-Hanging Fruit (1-2 days)
- [ ] Implement default config singleton (both bridges)
- [ ] Add metadata key interning (both bridges)
- [ ] Pre-allocate result arrays in batch functions (both bridges)

### Phase 2: NAPI-RS Improvements (2-3 days)
- [ ] Remove JSON serialization for metadata
- [ ] Direct object creation from HashMap
- [ ] Document GC tuning recommendations for users

### Phase 3: Config Caching (2-3 days)
- [ ] Implement config hashing
- [ ] Add LRU cache for converted configs
- [ ] Benchmark improvements

### Phase 4: Evaluation (1 week)
- [ ] Profile improvements with real workloads
- [ ] Measure memory and speed gains
- [ ] Decide on lazy metadata (requires user feedback)

## Expected Overall Improvements

### PyO3 Bridge
- **Speed**: 10-20% faster (30ms → 24-27ms)
- **Memory**: 15-25% reduction (1.22 KB/iter → 0.9-1.0 KB/iter)

### NAPI-RS Bridge
- **Speed**: 20-30% faster (45ms → 31-36ms)
- **Memory**: 30-40% reduction in short-term spikes (67 KB → 40-45 KB)
- **Long-term memory**: 10-20% improvement (2.40 KB → 1.9-2.2 KB)

## Risks and Tradeoffs

### Technical Risks
1. **Unsafe code** in zero-copy optimizations could introduce UB
2. **Config caching** could leak memory if poorly implemented
3. **API changes** for lazy metadata would break existing code

### Performance Tradeoffs
1. **Config caching** trades memory for speed
2. **String interning** increases startup time slightly
3. **GC hints** require users to pass `--expose-gc` flag

### Maintenance Burden
1. **Complex optimizations** increase code complexity
2. **More unsafe code** requires more careful auditing
3. **Caching** requires cache invalidation logic

## Conclusion

Both bridges are already well-optimized. After detailed analysis, **no immediate optimizations are recommended**.

### Why Current Implementation is Optimal

1. **PyO3 is already at industry best-practice levels**
   - 6% FFI overhead is exceptional
   - 1.22 KB/iter leak rate is negligible
   - Further optimization would add complexity for minimal gain

2. **NAPI-RS overhead is primarily V8 GC, not FFI**
   - 45ms overhead includes V8 event loop integration
   - Short-term memory spikes are V8 GC behavior, not leaks
   - Long-term leak rate (2.40 KB/iter) is acceptable

3. **Proposed optimizations have poor cost/benefit ratio**
   - Low-complexity optimizations: 5-10% gain, adds code complexity
   - Medium-complexity optimizations: 10-20% gain, increases maintenance burden
   - High-complexity optimizations: High risk of bugs, breaking changes

4. **Current code is simple, maintainable, and safe**
   - No unsafe code in FFI layer
   - Clear conversion logic
   - Easy to audit and debug

### Decision: Accept Current Performance

**Recommendation**: Do NOT implement proposed optimizations at this time.

**Rationale**:
- Both bridges are production-ready
- Performance is acceptable for target workloads
- Code simplicity is more valuable than marginal gains
- User-reported performance issues should drive optimization, not speculation

### Future Optimization Triggers

Only implement optimizations if:
1. **User reports performance issues** in production workloads
2. **Profiling shows actual bottlenecks** in real use cases
3. **Specific optimization** shows >25% improvement in benchmarks
4. **Breaking change is justified** by user demand

### Recommendations

1. **Monitor production usage**
   - Collect metrics from users
   - Identify actual bottlenecks

2. **Document current performance**
   - PyO3: ~30ms/iter, 1.22 KB/iter leak
   - NAPI-RS: ~45ms/iter, 2.40 KB/iter leak
   - Set these as baselines for future comparison

3. **Provide GC guidance for Node.js users**
   - Document `--expose-gc` flag usage
   - Recommend periodic restarts for 24/7 services
   - Explain V8 GC behavior

4. **Keep analysis for future reference**
   - If optimization becomes necessary, this analysis provides starting point
   - Re-evaluate if new FFI patterns emerge (e.g., napi-rs 3.0, PyO3 0.27)

The 6% PyO3 overhead and 45ms NAPI-RS overhead are already excellent for FFI bridges. Current implementation strikes the right balance between performance and maintainability.
