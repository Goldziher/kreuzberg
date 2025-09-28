# Framework Comparison

Technical comparison of document extraction frameworks included in benchmarks.

## Framework Specifications

### Kreuzberg v4

**Architecture**: Pure Python with optional Rust components
**Document Support**: 17 format types
**Installation Size**: ~80MB
**Dependencies**: Minimal core, optional extras

**Capabilities**:

- Synchronous and asynchronous APIs
- Multiple OCR backends (Tesseract, EasyOCR, PaddleOCR)
- Structured data extraction
- Language detection
- Quality assessment metrics

**Limitations**:

- MSG format not supported
- OCR quality depends on backend selection

### Kreuzberg v3

**Architecture**: Pure Python implementation
**Document Support**: 15 format types
**Installation Size**: ~70MB
**Dependencies**: Core libraries only

**Capabilities**:

- Mature, stable API
- Comprehensive format support
- Well-tested extraction algorithms
- Lightweight installation

**Limitations**:

- Synchronous processing only
- Limited structured data extraction
- No built-in quality metrics

### Extractous

**Architecture**: Rust core with Python bindings
**Document Support**: 1000+ formats via Apache Tika
**Installation Size**: ~50MB
**Dependencies**: Native binary components

**Capabilities**:

- High-performance Rust implementation
- Extensive format coverage
- Memory-efficient processing
- Cross-platform binary distribution

**Limitations**:

- Limited metadata extraction
- Binary dependency management
- Reduced customization options

### Unstructured

**Architecture**: Python with ML model integration
**Document Support**: 64+ format types
**Installation Size**: ~150MB
**Dependencies**: Machine learning libraries

**Capabilities**:

- Enterprise-focused feature set
- Email and complex document handling
- Element classification
- Production-ready scaling

**Limitations**:

- Large installation footprint
- Complex dependency tree
- Resource-intensive processing

### MarkItDown

**Architecture**: Python with ONNX Runtime
**Document Support**: Office and web formats
**Installation Size**: ~120MB
**Dependencies**: ONNX Runtime, Office libraries

**Capabilities**:

- Markdown-optimized output
- LLM preprocessing focus
- Microsoft Office integration
- ML-enhanced extraction

**Limitations**:

- Limited format coverage
- Markdown-centric design
- ONNX Runtime overhead

### Docling

**Architecture**: Python with deep learning models
**Document Support**: 10 core formats
**Installation Size**: ~1GB
**Dependencies**: PyTorch, Transformers, ML models

**Capabilities**:

- Advanced document understanding
- Layout analysis
- Table extraction
- Research-grade algorithms

**Limitations**:

- Large installation size
- Limited format support
- High resource requirements
- Complex model management

## Performance Characteristics

### Processing Speed

Typical performance patterns observed:

- **Extractous**: Fastest for supported formats due to Rust implementation
- **Kreuzberg v4**: Balanced performance across format types
- **Kreuzberg v3**: Consistent speed with lower overhead
- **Unstructured**: Moderate speed with comprehensive features
- **MarkItDown**: Variable speed depending on content complexity
- **Docling**: Slower due to ML model inference overhead

### Memory Usage

Resource consumption patterns:

- **Extractous**: Lowest memory footprint
- **Kreuzberg v3/v4**: Moderate memory usage
- **MarkItDown**: ONNX Runtime memory overhead
- **Unstructured**: High memory for complex documents
- **Docling**: Highest memory due to ML models

### Format Coverage

Support matrix by document type:

| Format | Kreuzberg v4 | Kreuzberg v3 | Extractous | Unstructured | MarkItDown | Docling |
| ------ | ------------ | ------------ | ---------- | ------------ | ---------- | ------- |
| PDF    | ✓            | ✓            | ✓          | ✓            | ✓          | ✓       |
| DOCX   | ✓            | ✓            | ✓          | ✓            | ✓          | ✓       |
| PPTX   | ✓            | ✓            | ✓          | ✓            | ✓          | ✓       |
| XLSX   | ✓            | ✓            | ✓          | ✓            | ✓          | ✗       |
| HTML   | ✓            | ✓            | ✓          | ✓            | ✓          | ✗       |
| Images | ✓            | ✓            | ✓          | ✓            | ✓          | ✓       |
| Email  | ✗            | ✗            | ✓          | ✓            | ✗          | ✗       |
| CSV    | ✓            | ✓            | ✓          | ✓            | ✗          | ✗       |

## Quality Assessment

### Text Extraction Accuracy

Framework-specific quality characteristics:

- **Docling**: Highest accuracy for complex layouts
- **Kreuzberg v4**: Balanced accuracy across formats
- **Unstructured**: Good for structured documents
- **Extractous**: Consistent but basic extraction
- **Kreuzberg v3**: Reliable text extraction
- **MarkItDown**: Optimized for markdown conversion

### Metadata Preservation

Metadata extraction capabilities:

- **Unstructured**: Comprehensive metadata support
- **Kreuzberg v4**: Good metadata extraction
- **Docling**: Research-focused metadata
- **Kreuzberg v3**: Basic metadata support
- **Extractous**: Limited metadata extraction
- **MarkItDown**: Minimal metadata focus

## Use Case Recommendations

### High-Performance Requirements

**Recommended**: Extractous, Kreuzberg v4
**Characteristics**: Fast processing, efficient resource usage
**Trade-offs**: May sacrifice some accuracy for speed

### Maximum Accuracy Needs

**Recommended**: Docling, Unstructured
**Characteristics**: Advanced ML-based extraction
**Trade-offs**: Higher resource usage and installation size

### Balanced Performance

**Recommended**: Kreuzberg v4, Kreuzberg v3
**Characteristics**: Good speed/accuracy balance
**Trade-offs**: Moderate capabilities across metrics

### Minimal Dependencies

**Recommended**: Kreuzberg v3, Extractous
**Characteristics**: Lightweight installation
**Trade-offs**: Reduced feature set

### Format Coverage Priority

**Recommended**: Extractous, Unstructured
**Characteristics**: Extensive format support
**Trade-offs**: May have inconsistent quality across formats

## Technical Considerations

### Installation Complexity

Framework installation requirements:

- **Kreuzberg v3/v4**: Standard Python package
- **Extractous**: Binary wheels available
- **MarkItDown**: ONNX Runtime dependency
- **Unstructured**: Multiple system dependencies
- **Docling**: PyTorch and model downloads

### API Design

Programming interface characteristics:

- **Kreuzberg v4**: Async/sync dual API
- **Kreuzberg v3**: Simple synchronous API
- **Extractous**: Pythonic wrapper over Rust
- **Unstructured**: Object-oriented design
- **MarkItDown**: Functional interface
- **Docling**: Research-oriented API

### Maintenance Status

Development activity and support:

- **Kreuzberg v4**: Active development
- **Kreuzberg v3**: Maintenance mode
- **Extractous**: Early stage, active
- **Unstructured**: Commercial backing
- **MarkItDown**: Microsoft support
- **Docling**: IBM Research project

## Integration Patterns

### Deployment Scenarios

Framework suitability by environment:

- **Cloud Serverless**: Kreuzberg v3/v4, Extractous
- **Container Deployment**: All frameworks supported
- **Edge Computing**: Kreuzberg v3, Extractous
- **High-Throughput**: Extractous, Kreuzberg v4
- **Research/Analysis**: Docling, Unstructured

### Scaling Considerations

Multi-document processing:

- **Memory Management**: Framework-specific patterns
- **Concurrency Support**: Async vs threading models
- **Resource Cleanup**: Garbage collection behavior
- **Error Recovery**: Robustness under load
