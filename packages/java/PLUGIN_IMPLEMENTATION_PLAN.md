# Java Plugin Registration Implementation Plan

## Current Status (End of Session 1)

### ✅ Completed
- All core extraction APIs implemented:
  - `extractFileSync()` - multiple overloads
  - `extractBytesSync()`
  - `batchExtractFilesSync()`
  - `batchExtractBytesSync()`
- 16 unit tests passing
- OCR backend registration working
- Functional parity for 90% of use cases

### ⏳ Remaining for Full 1:1 Parity
- PostProcessor registration with callbacks
- Validator registration with callbacks
- E2E test suite matching Python/Ruby/TypeScript coverage

---

## Implementation Plan for Session 2+

### Phase 1: FFI Layer (Rust)

**File**: `crates/kreuzberg-ffi/src/lib.rs`

#### 1.1 PostProcessor FFI Functions

```rust
// Function pointer types for Java callbacks
type PostProcessorCallback = unsafe extern "C" fn(
    processor_handle: *const c_void,
    result_json: *const c_char,
) -> *mut c_char;

#[repr(C)]
pub struct CPostProcessor {
    handle: *const c_void,  // Java object reference
    callback: PostProcessorCallback,
    name: *const c_char,
}

#[no_mangle]
pub unsafe extern "C" fn kreuzberg_register_post_processor(
    processor: *const CPostProcessor,
) -> bool {
    // 1. Validate inputs
    // 2. Create Rust PostProcessor wrapper that calls Java callback
    // 3. Register with kreuzberg registry
    // 4. Return success/failure
}

#[no_mangle]
pub unsafe extern "C" fn kreuzberg_unregister_post_processor(
    name: *const c_char,
) -> bool {
    // 1. Unregister from registry
    // 2. Cleanup
}
```

#### 1.2 Validator FFI Functions

```rust
type ValidatorCallback = unsafe extern "C" fn(
    validator_handle: *const c_void,
    result_json: *const c_char,
) -> *mut c_char; // Returns error message or NULL

#[repr(C)]
pub struct CValidator {
    handle: *const c_void,
    callback: ValidatorCallback,
    name: *const c_char,
}

#[no_mangle]
pub unsafe extern "C" fn kreuzberg_register_validator(
    validator: *const CValidator,
    priority: i32,
) -> bool {
    // Similar to PostProcessor
}

#[no_mangle]
pub unsafe extern "C" fn kreuzberg_unregister_validator(
    name: *const c_char,
) -> bool {
    // Unregister and cleanup
}
```

#### 1.3 Rust Wrapper Implementations

```rust
struct JavaPostProcessor {
    handle: *const c_void,
    callback: PostProcessorCallback,
    name: String,
}

unsafe impl Send for JavaPostProcessor {}
unsafe impl Sync for JavaPostProcessor {}

#[async_trait]
impl PostProcessor for JavaPostProcessor {
    async fn process(
        &self,
        result: &mut ExtractionResult,
        _config: &ExtractionConfig,
    ) -> Result<()> {
        // 1. Serialize ExtractionResult to JSON
        // 2. Call Java callback via function pointer
        // 3. Deserialize modified result from JSON
        // 4. Update result
    }
}
```

---

### Phase 2: Java Layer (Panama FFI)

**File**: `src/main/java/dev/kreuzberg/Kreuzberg.java`

#### 2.1 Method Handle Setup

```java
// Callback descriptor for PostProcessor
private static final FunctionDescriptor POST_PROCESSOR_DESC = FunctionDescriptor.of(
    ADDRESS,  // return: char* (modified result JSON)
    ADDRESS,  // processor_handle
    ADDRESS   // result_json
);

// Callback descriptor for Validator
private static final FunctionDescriptor VALIDATOR_DESC = FunctionDescriptor.of(
    ADDRESS,  // return: char* (error message or NULL)
    ADDRESS,  // validator_handle
    ADDRESS   // result_json
);
```

#### 2.2 Registration Methods

```java
public static void registerPostProcessor(String name, PostProcessor processor) {
    // 1. Create upcall stub from PostProcessor.process() method
    MethodHandle processMH = /* create from processor.process() */;
    MemorySegment callback = LINKER.upcallStub(
        processMH,
        POST_PROCESSOR_DESC,
        Arena.global()
    );

    // 2. Store processor in Java-side registry to prevent GC
    POST_PROCESSORS.put(name, processor);

    // 3. Call FFI function
    kreuzberg_register_post_processor(callback, name);
}

public static void unregisterPostProcessor(String name) {
    POST_PROCESSORS.remove(name);
    kreuzberg_unregister_post_processor(name);
}

// Similar for Validator
public static void registerValidator(String name, Validator validator, int priority) {
    // Similar pattern
}
```

#### 2.3 Java-side Processor Storage

```java
// Prevent GC of Java processors while registered
private static final Map<String, PostProcessor> POST_PROCESSORS = new ConcurrentHashMap<>();
private static final Map<String, Validator> VALIDATORS = new ConcurrentHashMap<>();
```

---

### Phase 3: Updated Interfaces

#### 3.1 Enhanced PostProcessor Interface

```java
@FunctionalInterface
public interface PostProcessor {
    ExtractionResult process(ExtractionResult result) throws KreuzbergException;

    // Optional lifecycle methods (for compatibility)
    default String name() { return getClass().getSimpleName(); }
    default void initialize() throws Exception {}
    default void shutdown() throws Exception {}
}
```

#### 3.2 Enhanced Validator Interface

```java
@FunctionalInterface
public interface Validator {
    void validate(ExtractionResult result) throws ValidationException;

    default String name() { return getClass().getSimpleName(); }
}
```

---

### Phase 4: Testing

#### 4.1 Unit Tests

**File**: `src/test/java/dev/kreuzberg/PluginTest.java`

```java
@Test
void testPostProcessorRegistration() {
    PostProcessor uppercase = result ->
        result.withContent(result.content().toUpperCase());

    Kreuzberg.registerPostProcessor("uppercase", uppercase);

    ExtractionResult result = Kreuzberg.extractFileSync("test.txt");
    assertTrue(result.content().equals(result.content().toUpperCase()));

    Kreuzberg.unregisterPostProcessor("uppercase");
}

@Test
void testValidatorRegistration() {
    Validator minLength = result -> {
        if (result.content().length() < 10) {
            throw new ValidationException("Too short");
        }
    };

    Kreuzberg.registerValidator("minLength", minLength, 100);

    assertThrows(ValidationException.class, () -> {
        Kreuzberg.extractFileSync("short.txt");
    });

    Kreuzberg.unregisterValidator("minLength");
}
```

#### 4.2 E2E Tests

Match Python/Ruby/TypeScript E2E test structure:
- Create `src/test/java/dev/kreuzberg/E2ETest.java`
- Test fixtures from `test_documents/`
- Similar assertions for content, tables, metadata

---

## Key Technical Considerations

### 1. Memory Safety
- Java processors stored in ConcurrentHashMap prevent GC
- Panama FFI arenas manage upcall stub lifecycle
- Rust side uses Arc<dyn PostProcessor> for thread safety

### 2. Thread Safety
- All FFI functions are `unsafe extern "C"`
- Rust wrappers implement Send + Sync
- Java uses ConcurrentHashMap for processor storage

### 3. Error Handling
- Java exceptions → JSON error messages → Rust KreuzbergError
- Rust errors → set_last_error() → Java throws KreuzbergException

### 4. JSON Serialization
- ExtractionResult ↔ JSON for callback communication
- Use serde_json on Rust side
- Use Jackson on Java side (already in dependencies)

---

## Success Criteria

- [ ] PostProcessor registration works
- [ ] Validator registration works
- [ ] All unit tests pass (target: 25+ tests)
- [ ] E2E tests match Python/Ruby/TypeScript coverage
- [ ] No memory leaks or segfaults
- [ ] Thread-safe under concurrent access

---

## Estimated Effort

- Phase 1 (FFI Layer): 2-3 hours
- Phase 2 (Java Layer): 1-2 hours
- Phase 3 (Interfaces): 30 minutes
- Phase 4 (Testing): 2-3 hours

**Total**: 6-9 hours across 2-3 sessions

---

## References

- Panama FFI upcalls: https://docs.oracle.com/en/java/javase/22/core/foreign-function-and-memory-api.html#GUID-923F44E6-C357-4C92-962F-5EF746279C90
- Python implementation: `crates/kreuzberg-py/src/plugins.rs`
- Ruby implementation: `packages/ruby/ext/kreuzberg_rb/native/src/lib.rs`
- TypeScript implementation: `crates/kreuzberg-node/src/lib.rs`
