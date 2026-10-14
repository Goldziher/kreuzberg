use super::*;

#[tokio::test]
#[serial]
async fn test_pipeline_metadata_added_in_early_visible_in_middle() {
    clear_processor_registry();

    let registry = get_post_processor_registry();
    {
        let mut reg = registry.write();

        let early = Arc::new(MetadataAddingProcessor {
            name: "early".to_string(),
            key: "early_key".to_string(),
            value: "early_value".to_string(),
        });

        struct MiddleReadingProcessor;
        impl Plugin for MiddleReadingProcessor {
            fn name(&self) -> &str {
                "middle"
            }
            fn version(&self) -> String {
                "1.0.0".to_string()
            }
            fn initialize(&self) -> Result<()> {
                Ok(())
            }
            fn shutdown(&self) -> Result<()> {
                Ok(())
            }
        }
        #[async_trait]
        impl PostProcessor for MiddleReadingProcessor {
            async fn process(&self, result: &mut ExtractedDocument, _: &ExtractionConfig) -> Result<()> {
                if let Some(val) = result.metadata.additional.get(&Cow::Borrowed("early_key")) {
                    result
                        .metadata
                        .additional
                        .insert(Cow::Borrowed("middle_saw"), val.clone());
                }
                Ok(())
            }
            fn processing_stage(&self) -> ProcessingStage {
                ProcessingStage::Middle
            }
        }

        reg.register(early).expect("Operation failed");
        reg.register(Arc::new(MiddleReadingProcessor))
            .expect("Operation failed");
    }

    let doc = mock_doc("content");
    let config = ExtractionConfig::default();

    let processed = run_pipeline(doc, &config).await.expect("Async operation failed");
    assert_eq!(
        processed
            .metadata
            .additional
            .get("middle_saw")
            .expect("Operation failed")
            .as_str()
            .expect("Operation failed"),
        "early_value"
    );
}

#[tokio::test]
#[serial]
async fn test_pipeline_content_modified_in_middle_visible_in_late() {
    clear_processor_registry();

    let registry = get_post_processor_registry();
    {
        let mut reg = registry.write();

        let middle = Arc::new(OrderTrackingProcessor::new("middle-content", ProcessingStage::Middle));

        struct LateReadingProcessor;
        impl Plugin for LateReadingProcessor {
            fn name(&self) -> &str {
                "late"
            }
            fn version(&self) -> String {
                "1.0.0".to_string()
            }
            fn initialize(&self) -> Result<()> {
                Ok(())
            }
            fn shutdown(&self) -> Result<()> {
                Ok(())
            }
        }
        #[async_trait]
        impl PostProcessor for LateReadingProcessor {
            async fn process(&self, result: &mut ExtractedDocument, _: &ExtractionConfig) -> Result<()> {
                result.content.push_str("[late-saw-middle]");
                Ok(())
            }
            fn processing_stage(&self) -> ProcessingStage {
                ProcessingStage::Late
            }
        }

        reg.register(middle).expect("Operation failed");
        reg.register(Arc::new(LateReadingProcessor)).expect("Operation failed");
    }

    let doc = mock_doc("start");
    let config = ExtractionConfig::default();

    let processed = run_pipeline(doc, &config).await.expect("Async operation failed");
    assert!(processed.content.contains("[middle-content]"));
    assert!(processed.content.contains("[late-saw-middle]"));
}

#[tokio::test]
#[serial]
async fn test_pipeline_multiple_processors_modifying_same_metadata() {
    clear_processor_registry();

    let registry = get_post_processor_registry();
    {
        let mut reg = registry.write();

        for i in 1..=3 {
            struct MetadataOverwritingProcessor {
                name: String,
                value: String,
                priority: i32,
            }
            impl Plugin for MetadataOverwritingProcessor {
                fn name(&self) -> &str {
                    &self.name
                }
                fn version(&self) -> String {
                    "1.0.0".to_string()
                }
                fn initialize(&self) -> Result<()> {
                    Ok(())
                }
                fn shutdown(&self) -> Result<()> {
                    Ok(())
                }
            }
            #[async_trait]
            impl PostProcessor for MetadataOverwritingProcessor {
                async fn process(&self, result: &mut ExtractedDocument, _: &ExtractionConfig) -> Result<()> {
                    result
                        .metadata
                        .additional
                        .insert(Cow::Borrowed("shared_key"), serde_json::json!(self.value));
                    Ok(())
                }
                fn processing_stage(&self) -> ProcessingStage {
                    ProcessingStage::Early
                }
                fn priority(&self) -> i32 {
                    self.priority
                }
            }

            let processor = Arc::new(MetadataOverwritingProcessor {
                name: format!("proc{}", i),
                value: format!("value{}", i),
                priority: 100 - i * 10,
            });
            reg.register(processor).expect("Operation failed");
        }
    }

    let doc = mock_doc("content");
    let config = ExtractionConfig::default();

    let processed = run_pipeline(doc, &config).await.expect("Async operation failed");
    assert_eq!(
        processed
            .metadata
            .additional
            .get("shared_key")
            .expect("Operation failed")
            .as_str()
            .expect("Operation failed"),
        "value3"
    );
}

#[tokio::test]
#[serial]
async fn test_pipeline_processors_reading_previous_output() {
    clear_processor_registry();

    let registry = get_post_processor_registry();
    {
        let mut reg = registry.write();

        struct CountingProcessor {
            name: String,
            stage: ProcessingStage,
        }
        impl Plugin for CountingProcessor {
            fn name(&self) -> &str {
                &self.name
            }
            fn version(&self) -> String {
                "1.0.0".to_string()
            }
            fn initialize(&self) -> Result<()> {
                Ok(())
            }
            fn shutdown(&self) -> Result<()> {
                Ok(())
            }
        }
        #[async_trait]
        impl PostProcessor for CountingProcessor {
            async fn process(&self, result: &mut ExtractedDocument, _: &ExtractionConfig) -> Result<()> {
                let current_count = result
                    .metadata
                    .additional
                    .get(&Cow::Borrowed("count"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                result
                    .metadata
                    .additional
                    .insert(Cow::Borrowed("count"), serde_json::json!(current_count + 1));
                Ok(())
            }
            fn processing_stage(&self) -> ProcessingStage {
                self.stage
            }
        }

        for (name, stage) in [
            ("early1", ProcessingStage::Early),
            ("early2", ProcessingStage::Early),
            ("middle1", ProcessingStage::Middle),
            ("late1", ProcessingStage::Late),
        ] {
            let processor = Arc::new(CountingProcessor {
                name: name.to_string(),
                stage,
            });
            reg.register(processor).expect("Operation failed");
        }
    }

    let doc = mock_doc("content");
    let config = ExtractionConfig::default();

    let processed = run_pipeline(doc, &config).await.expect("Async operation failed");
    assert_eq!(
        processed
            .metadata
            .additional
            .get("count")
            .expect("Operation failed")
            .as_i64()
            .expect("Operation failed"),
        4
    );
}

#[tokio::test]
#[serial]
async fn test_pipeline_large_content_modification() {
    clear_processor_registry();

    struct LargeContentProcessor;
    impl Plugin for LargeContentProcessor {
        fn name(&self) -> &str {
            "large"
        }
        fn version(&self) -> String {
            "1.0.0".to_string()
        }
        fn initialize(&self) -> Result<()> {
            Ok(())
        }
        fn shutdown(&self) -> Result<()> {
            Ok(())
        }
    }
    #[async_trait]
    impl PostProcessor for LargeContentProcessor {
        async fn process(&self, result: &mut ExtractedDocument, _: &ExtractionConfig) -> Result<()> {
            result.content.push_str(&"x".repeat(10000));
            Ok(())
        }
        fn processing_stage(&self) -> ProcessingStage {
            ProcessingStage::Early
        }
    }

    let registry = get_post_processor_registry();
    {
        let mut reg = registry.write();
        reg.register(Arc::new(LargeContentProcessor)).expect("Operation failed");
    }

    let doc = mock_doc("start");
    let config = ExtractionConfig::default();

    let processed = run_pipeline(doc, &config).await.expect("Async operation failed");
    assert!(processed.content.len() > 10000);
}

#[tokio::test]
#[serial]
async fn test_pipeline_enabled_processors_whitelist() {
    clear_processor_registry();

    let registry = get_post_processor_registry();
    {
        let mut reg = registry.write();

        for name in ["proc1", "proc2", "proc3"] {
            let processor = Arc::new(OrderTrackingProcessor::new(name, ProcessingStage::Early));
            reg.register(processor).expect("Operation failed");
        }
    }

    let doc = mock_doc("start");
    let config = ExtractionConfig {
        postprocessor: Some(PostProcessorConfig {
            enabled: true,
            enabled_processors: Some(vec!["proc1".to_string(), "proc3".to_string()]),
            disabled_processors: None,
            enabled_set: None,
            disabled_set: None,
        }),
        ..Default::default()
    };

    let processed = run_pipeline(doc, &config).await.expect("Async operation failed");
    assert!(processed.content.contains("[proc1]"));
    assert!(!processed.content.contains("[proc2]"));
    assert!(processed.content.contains("[proc3]"));
}

#[tokio::test]
#[serial]
async fn test_pipeline_disabled_processors_blacklist() {
    clear_processor_registry();

    let registry = get_post_processor_registry();
    {
        let mut reg = registry.write();

        for name in ["proc1", "proc2", "proc3"] {
            let processor = Arc::new(OrderTrackingProcessor::new(name, ProcessingStage::Early));
            reg.register(processor).expect("Operation failed");
        }
    }

    let doc = mock_doc("start");
    let config = ExtractionConfig {
        postprocessor: Some(PostProcessorConfig {
            enabled: true,
            enabled_processors: None,
            disabled_processors: Some(vec!["proc2".to_string()]),
            enabled_set: None,
            disabled_set: None,
        }),
        ..Default::default()
    };

    let processed = run_pipeline(doc, &config).await.expect("Async operation failed");
    assert!(processed.content.contains("[proc1]"));
    assert!(!processed.content.contains("[proc2]"));
    assert!(processed.content.contains("[proc3]"));
}

#[tokio::test]
#[serial]
async fn test_pipeline_no_filtering_runs_all() {
    clear_processor_registry();

    let registry = get_post_processor_registry();
    {
        let mut reg = registry.write();

        for name in ["proc1", "proc2", "proc3"] {
            let processor = Arc::new(OrderTrackingProcessor::new(name, ProcessingStage::Early));
            reg.register(processor).expect("Operation failed");
        }
    }

    let doc = mock_doc("start");
    let config = ExtractionConfig::default();

    let processed = run_pipeline(doc, &config).await.expect("Async operation failed");
    assert!(processed.content.contains("[proc1]"));
    assert!(processed.content.contains("[proc2]"));
    assert!(processed.content.contains("[proc3]"));
}

#[tokio::test]
#[serial]
async fn test_pipeline_empty_whitelist_runs_none() {
    clear_processor_registry();

    let registry = get_post_processor_registry();
    {
        let mut reg = registry.write();

        for name in ["proc1", "proc2"] {
            let processor = Arc::new(OrderTrackingProcessor::new(name, ProcessingStage::Early));
            reg.register(processor).expect("Operation failed");
        }
    }

    let doc = mock_doc("start");
    let config = ExtractionConfig {
        postprocessor: Some(PostProcessorConfig {
            enabled: true,
            enabled_processors: Some(vec![]),
            disabled_processors: None,
            enabled_set: None,
            disabled_set: None,
        }),
        ..Default::default()
    };

    let processed = run_pipeline(doc, &config).await.expect("Async operation failed");
    assert_eq!(processed.content, "start");
}
