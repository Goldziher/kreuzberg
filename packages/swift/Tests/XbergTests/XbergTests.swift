import RustBridge
import XCTest

@testable import Xberg

final class XbergTests: XCTestCase {
  /// Round-trips the generated `CacheStats` DTO through `JSONEncoder`/`JSONDecoder`,
  /// so a broken `Codable` conformance or a field that silently stops encoding fails
  /// `swift test` immediately instead of shipping green with a suite that asserts
  /// nothing about the generated API. Create-only scaffold seed. ~keep
  func testCacheStatsRoundTripsThroughJSON() throws {
    let original = CacheStats(
      totalFiles: 1, totalSizeMb: 1.5, availableSpaceMb: 1.5, oldestFileAgeDays: 1.5,
      newestFileAgeDays: 1.5)
    let data = try JSONEncoder().encode(original)
    let decoded = try JSONDecoder().decode(CacheStats.self, from: data)
    XCTAssertEqual(decoded, original)
  }

  func testLegacyConfigInitializersRemainSourceCompatible() throws {
    let chunkerType = try ChunkerType.text.intoRust()
    let chunkSizing = try ChunkSizing.characters.intoRust()
    let tableChunking = try TableChunkingMode.split.intoRust()
    _ = ChunkingConfig(
      512, 32, true, chunkerType, nil, nil, nil, nil as String?, chunkSizing, nil, tableChunking)

    let embeddingModel = try embeddingModelTypeFromJson(
      #"{"type":"preset","name":"gte-modernbert-base"}"#)
    let embedding = EmbeddingConfig(embeddingModel, true, 8, false, nil as String?, nil, nil, nil)
    XCTAssertTrue(embedding.model().toString().contains("gte-modernbert-base"))

    let lateInteractionModel = try LateInteractionModelType.preset(name: "gte-moderncolbert")
    .intoRust()
    let lateInteraction = LateInteractionConfig(
      lateInteractionModel, 8, 512, 64, false, nil as String?, nil, nil)
    XCTAssertTrue(lateInteraction.model().toString().contains("gte-moderncolbert"))

    let nerBackend = try NerBackendKind.llm.intoRust()
    let nerCategories = RustVec<RustBridge.EntityCategory>()
    nerCategories.push(value: try EntityCategory.custom(field0: "legal_party").intoRust())
    let ner = NerConfig(
      nerBackend,
      nerCategories,
      nil as RustString?,
      nil,
      RustVec<RustString>()
    )
    XCTAssertEqual(ner.backend().toString(), "llm")
    XCTAssertTrue(
      ner.categories().map { $0.as_str().toString() }.contains("{\"custom\":\"legal_party\"}"))

    let redactionStrategy = try RedactionStrategy.mask.intoRust()
    let offsetEncoding = try RedactionOffsetEncoding.utf8Bytes.intoRust()
    _ = RedactionConfig(
      RustVec<RustBridge.PiiCategory>(),
      redactionStrategy,
      nil,
      true,
      RustVec<RustBridge.RedactionTerm>(),
      RustVec<RustBridge.RedactionPattern>(),
      RustVec<RustBridge.ExternalRedactionFinding>(),
      nil,
      nil as String?,
      offsetEncoding
    )

    let rerankerModel = try rerankerModelTypeFromJson(#"{"type":"preset","name":"balanced"}"#)
    let reranker = RerankerConfig(rerankerModel, nil, 8, false, nil as String?, nil, nil)
    XCTAssertTrue(reranker.model().toString().contains("balanced"))

    let sparseModel = try SparseEmbeddingModelType.preset(name: "opensearch-v3-distill").intoRust()
    let sparse = SparseEmbeddingConfig(sparseModel, 8, 512, false, nil as String?, nil, nil)
    XCTAssertTrue(sparse.model().toString().contains("opensearch-v3-distill"))
  }
}
