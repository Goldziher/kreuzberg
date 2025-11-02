# FFI Patterns Analysis

## Summary

Investigating why post-processors don't run when validators are also registered via FFI (TypeScript/Python).

## Evidence

### Working
- ✅ **Rust core tests**: All pass - `test_postprocessor_runs_before_validator()` works
- ✅ **Post-processor only (TS)**: Works - `test-postprocessor-only.mjs` passes
- ✅ **Validator only**: Would likely work (not tested yet)

### Failing
- ❌ **Post-processor + Validator (TS)**: `test-validator-debug.mjs` fails
  - Execution order: `['validator']` (post-processor never runs)
  - `result.metadata.processed = undefined`
- ❌ **Post-processor + Validator (Python)**: `test_python_pipeline_bug.py` fails
  - Same symptoms as TypeScript

### Debug Findings
1. **No Rust `eprintln!` logs appear**: `[PIPELINE]` and `[POST-PROCESSOR]` debug logs don't show
2. **JavaScript `console.log` works**: "VALIDATOR called" appears
3. **Pipeline metadata debug flags missing**: `_debug_pipeline_called` not in final result
4. **But `quality_score` appears**: Added later in pipeline, proves pipeline runs

## NAPI-RS Patterns (from official examples)

### ThreadsafeFunction Usage

**Pattern from `/tmp/napi-rs/examples/napi/src/threadsafe_function.rs`**:

1. **Building from Function**:
```rust
let tsfn = func
    .build_threadsafe_function()
    .build_callback(|ctx| {
        // Transform arguments here
        Ok(vec![ctx.value])
    })?;
```

2. **Calling Async**:
```rust
let result = tsfn.call_async(Ok(value)).await?;
```

3. **Calling Non-blocking**:
```rust
tsfn.call(Ok(value), ThreadsafeFunctionCallMode::NonBlocking);
```

4. **Storing in Struct**:
```rust
struct Wrapper {
    tsfn: Arc<ThreadsafeFunction<Input, Output>>,
}
```

### Our Implementation Pattern

**Location**: `/Users/naamanhirschfeld/workspace/kreuzberg/crates/kreuzberg-node/src/lib.rs`

**Post-Processor Registration** (lines 1192-1257):
```rust
#[napi]
pub fn register_post_processor(_env: Env, processor: Object) -> Result<()> {
    // 1. Validate object has required methods
    validate_plugin_object(&processor, "PostProcessor", &["name", "process"])?;

    // 2. Extract name
    let name_fn: Function<(), String> = processor.get_named_property("name")?;
    let name: String = name_fn.call(())?;

    // 3. Extract stage (optional, defaults to Middle)
    let stage = if let Ok(stage_fn) = processor.get_named_property::<Function<(), String>>("processingStage") {
        // ... parse stage ...
    } else {
        ProcessingStage::Middle
    };

    // 4. Build ThreadsafeFunction from process method
    let process_fn: Function<String, Promise<String>> = processor.get_named_property("process")?;
    let tsfn = process_fn.build_threadsafe_function().build_callback(|ctx| {
        Ok(vec![ctx.value])
    })?;

    // 5. Create wrapper and register
    let js_processor = JsPostProcessor {
        name: name.clone(),
        process_fn: Arc::new(tsfn),
        stage,
    };

    let registry = get_post_processor_registry();
    let mut registry = registry.write()?;
    registry.register(Arc::new(js_processor), 0)?; // Priority = 0

    Ok(())
}
```

**Post-Processor Execution** (lines 1085-1150):
```rust
impl RustPostProcessor for JsPostProcessor {
    async fn process(
        &self,
        result: &mut kreuzberg::ExtractionResult,
        _config: &kreuzberg::ExtractionConfig,
    ) -> std::result::Result<(), kreuzberg::KreuzbergError> {
        // 1. Convert Rust → JS (serialize to JSON)
        let js_result = JsExtractionResult::try_from(result.clone())?;
        let json_input = serde_json::to_string(&js_result)?;

        // 2. Call JS function via ThreadsafeFunction
        let json_output = self.process_fn.call_async(json_input).await?.await?;

        // 3. Convert JS → Rust (deserialize from JSON)
        let updated: JsExtractionResult = serde_json::from_str(&json_output)?;
        let rust_result = kreuzberg::ExtractionResult::try_from(updated)?;

        // 4. Update result
        *result = rust_result;
        Ok(())
    }
}
```

**Validator Registration** (lines 1452-1512):
```rust
#[napi]
pub fn register_validator(_env: Env, validator: Object) -> Result<()> {
    // Very similar pattern to post-processor
    // ...
    let registry = get_validator_registry();
    let mut registry = registry.write()?;
    registry.register(Arc::new(js_validator))?; // Uses validator's priority()

    Ok(())
}
```

## Result Conversion

**Rust → JS** (lines 431-494):
- Serializes `Metadata` to `serde_json::Value`
- Preserves all fields including `metadata.additional`

**JS → Rust** (lines 495-625):
- Deserializes metadata as HashMap
- Extracts known fields (language, date, format_type, etc.)
- **Everything else goes into `additional`** (line 565)
- This preserves custom fields added by JS post-processors

## Architecture Flow

```
extractBytes(data, mime_type, config)
  ↓
kreuzberg::extract_bytes(&data, &mime_type, &config)  [Rust core]
  ↓
kreuzberg::extract_file() / extract_bytes()  [extractor.rs:116]
  ↓
kreuzberg::run_pipeline(result, config)  [pipeline.rs:33]
  ↓
  ├─ Run post-processors (lines 46-103)
  │   ├─ For each ProcessingStage (Early, Middle, Late)
  │   │   ├─ Get processors from registry
  │   │   └─ For each processor:
  │   │       ├─ Check should_process()
  │   │       └─ Call processor.process(&mut result, config)
  │   │           └─ For JS: JsPostProcessor::process()
  │   │               ├─ Serialize result to JSON
  │   │               ├─ Call JS via ThreadsafeFunction
  │   │               ├─ Deserialize result from JSON
  │   │               └─ Update result
  │   └─ [All post-processors complete]
  │
  ├─ Calculate quality score (lines 106-132)
  │
  ├─ Run chunking if enabled (lines 134-196)
  │
  └─ Run validators (lines 214-252)
      ├─ Get all validators from registry
      └─ For each validator:
          └─ Call validator.validate(result, config)
              └─ For JS: JsValidator::validate()
                  ├─ Serialize result to JSON
                  ├─ Call JS via ThreadsafeFunction
                  └─ Handle any errors
```

## The Mystery

**Question**: Why do post-processors not run when validators are also registered?

**Observations**:
1. Post-processors work fine alone
2. Validators likely work fine alone (not tested)
3. Together, only validators run
4. Rust core tests prove the pipeline logic is correct
5. Both use identical registration patterns
6. Both use same ThreadsafeFunction approach

**Theories**:
1. ❓ Registration order issue? (validator clears post-processors?)
2. ❓ Global registry corruption?
3. ❓ Config issue disabling post-processors when validators present?
4. ❓ Early return in pipeline before post-processors?
5. ❓ `should_process()` returning false?

## Key Findings from Registry Investigation

### ✅ Registration Works Correctly

Added `list_post_processors()` and `list_validators()` functions to verify registrations.

**Test Results** (`test-check-registrations.mjs`):
```
Before registration:
  Post-processors: []
  Validators: []

After registering post-processor:
  Post-processors: [ 'test-proc' ]
  Validators: []

After registering validator:
  Post-processors: [ 'test-proc' ]
  Validators: [ 'test-val' ]
```

**Conclusion**: ✅ Registration is working correctly
- Both plugins successfully register
- Validators do NOT clear post-processors
- Both remain in their respective registries

**This rules out theories 1 & 2**:
- ❌ NOT a registration order issue
- ❌ NOT registry corruption

## ROOT CAUSE IDENTIFIED ✅

### The Problem
The test failures were NOT a bug in the pipeline execution logic. The issue was with the **build process**.

### What Was Wrong
- Simply running `cargo build --release` and copying the `.dylib` file is **insufficient** for NAPI-RS
- NAPI-RS requires running `pnpm run build` to properly regenerate JavaScript bindings
- The NAPI-CLI generates the JavaScript wrapper (`index.js`) from Rust `#[napi]` macros
- Without proper regeneration, the JavaScript bindings were stale or incomplete

### The Solution
**Always use the proper NAPI build command**:
```bash
cd /Users/naamanhirschfeld/workspace/kreuzberg/crates/kreuzberg-node
pnpm run build
```

This command:
1. Runs `cargo build --release` to compile the Rust code
2. Uses NAPI-CLI to generate fresh JavaScript bindings from `#[napi]` macros
3. Properly links the native module to the TypeScript package

### Test Results After Proper Build
```
✓ Test passed!
Execution order: [ 'postprocessor', 'validator' ]
result.metadata.processed = true
```

Both post-processors and validators now run correctly in the proper order.

### Lessons Learned
1. **NAPI-RS build requirements**: Must use `pnpm run build`, not just `cargo build`
2. **Investigation value**: The diagnostic functions (`list_post_processors`, `list_validators`) were valuable for ruling out registry issues
3. **Root cause complexity**: What appeared to be a complex pipeline execution bug was actually a build/tooling issue

## Python FFI Status - ✅ FIXED

### Issue Identified

Python's `PostProcessor::process()` and `Validator::validate()` were using synchronous `Python::attach()` directly in `async fn` methods, blocking the Tokio async runtime and causing timing/ordering issues when both post-processors and validators were registered.

### Solution (crates/kreuzberg-py/src/plugins.rs:964-999, 1334-1376)

**Key Insight**: Use `tokio::task::block_in_place` instead of `spawn_blocking`

**Why This Works**:
- `spawn_blocking`: Moves task to separate thread pool → GIL deadlock ❌
- `block_in_place`: Runs blocking code on current thread, signals Tokio to spawn more workers → no GIL deadlock ✅

**Implementation Pattern**:
```rust
async fn process(&self, result: &mut ExtractionResult, _config: &ExtractionConfig) -> Result<()> {
    let processor_name = self.name.clone();

    // Use block_in_place to run blocking Python code without thread pool deadlock
    let updated_result = tokio::task::block_in_place(|| {
        Python::attach(|py| {
            // Python FFI operations...
            Ok::<ExtractionResult, KreuzbergError>(updated_result)
        })
    })?;

    *result = updated_result;
    Ok(())
}
```

**Test Results**:
```
✓ Test passed!
Execution order: ['postprocessor', 'validator']
Has quality_score: True
Has processed: True
```

Both post-processors and validators now run correctly in the proper order.

### Status Summary

- **TypeScript/NAPI**: ✅ Fixed by proper build process (`pnpm run build`)
- **Python/PyO3**: ✅ Fixed by using `block_in_place` instead of `spawn_blocking`

## Next Steps

1. ✅ ~~Add `list_post_processors()` and `list_validators()` functions~~ - DONE
2. ✅ ~~Verify registries are actually populated~~ - CONFIRMED
3. ✅ ~~Identify root cause for TypeScript~~ - BUILD PROCESS ISSUE
4. ✅ ~~Verify Python FFI~~ - PYTHON HAS REAL BUG
5. ✅ ~~Attempt fix with spawn_blocking~~ - COMPILED BUT CAUSES DEADLOCK
6. ✅ ~~Research PyO3 async patterns and fix deadlock~~ - FIXED WITH `block_in_place`
7. Clean up test files and debug logging
