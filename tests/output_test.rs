use serde_json::json;

/// Test truncate helper function
#[test]
fn test_truncate() {
    fn truncate(s: &str, max: usize) -> String {
        if s.len() <= max {
            s.to_string()
        } else {
            format!("{}...", &s[..max - 3])
        }
    }

    assert_eq!(truncate("hello", 10), "hello");
    assert_eq!(truncate("hello world this is long", 10), "hello w...");
    assert_eq!(truncate("abc", 3), "abc");
    assert_eq!(truncate("abcd", 3), "...");
}

/// Test JSON pretty printing
#[test]
fn test_json_pretty_print() {
    let value = json!({"key": "value", "number": 42});
    let pretty = serde_json::to_string_pretty(&value).unwrap();
    assert!(pretty.contains("\"key\""));
    assert!(pretty.contains("\"value\""));
    assert!(pretty.contains("42"));
}

/// Test extracting data from API response formats
#[test]
fn test_api_response_data_extraction() {
    // Models response with "data" wrapper
    let response = json!({
        "data": [
            {"id": "model-1", "provider": "openai", "name": "GPT-4"},
            {"id": "model-2", "provider": "anthropic", "name": "Claude"}
        ]
    });

    let data = response.get("data").unwrap();
    let arr = data.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["id"], "model-1");
    assert_eq!(arr[1]["provider"], "anthropic");
}

/// Test extracting results from patent search response
#[test]
fn test_patent_search_response_extraction() {
    let response = json!({
        "results": [
            {
                "publicationNumber": "EP1234567",
                "title": "Test Patent",
                "applicant": "Test Corp",
                "publicationDate": "2024-01-01"
            }
        ],
        "total": 1
    });

    let results = response.get("results").unwrap();
    let arr = results.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["publicationNumber"], "EP1234567");
    assert_eq!(arr[0]["title"], "Test Patent");
}

/// Test chat completion response parsing
#[test]
fn test_chat_completion_response() {
    let response = json!({
        "choices": [{
            "message": {
                "role": "assistant",
                "content": "This is the response."
            },
            "finish_reason": "stop"
        }]
    });

    let content = response
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str());

    assert_eq!(content, Some("This is the response."));
}

/// Test SSE streaming delta parsing
#[test]
fn test_sse_delta_parsing() {
    let chunk = json!({
        "choices": [{
            "delta": {
                "content": "Hello"
            }
        }]
    });

    let content = chunk
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("delta"))
        .and_then(|d| d.get("content"))
        .and_then(|c| c.as_str());

    assert_eq!(content, Some("Hello"));
}

/// Test SSE line parsing
#[test]
fn test_sse_line_parsing() {
    let sse_data = "data: {\"choices\":[{\"delta\":{\"content\":\"Hi\"}}]}\n\ndata: [DONE]\n\n";

    let mut events = Vec::new();
    let mut buffer = sse_data.to_string();

    while let Some(pos) = buffer.find("\n\n") {
        let event = buffer[..pos].to_string();
        buffer = buffer[pos + 2..].to_string();

        for line in event.lines() {
            if let Some(data) = line.strip_prefix("data: ") {
                events.push(data.to_string());
            }
        }
    }

    assert_eq!(events.len(), 2);
    assert!(events[0].contains("\"content\":\"Hi\""));
    assert_eq!(events[1], "[DONE]");
}

/// Test build-query response parsing
#[test]
fn test_build_query_response() {
    let response = json!({
        "query": "ti=solar AND ti=panel AND pa=Tesla",
        "explanation": "Searches for patents with 'solar' and 'panel' in the title filed by Tesla."
    });

    assert_eq!(
        response.get("query").and_then(|q| q.as_str()),
        Some("ti=solar AND ti=panel AND pa=Tesla")
    );
    assert!(response
        .get("explanation")
        .and_then(|e| e.as_str())
        .unwrap()
        .contains("Tesla"));
}

/// Test academic search response parsing
#[test]
fn test_academic_search_response() {
    let response = json!({
        "results": [
            {
                "title": "Machine Learning in Patent Analysis",
                "authors": "Smith, J.; Doe, A.",
                "year": 2024,
                "source": "Nature AI"
            }
        ]
    });

    let results = response.get("results").unwrap().as_array().unwrap();
    assert_eq!(results[0]["title"], "Machine Learning in Patent Analysis");
    assert_eq!(results[0]["year"], 2024);
}

/// Test OCR response parsing
#[test]
fn test_ocr_response() {
    let response = json!({
        "text": "This is extracted text from the document.",
        "pages": 3
    });

    assert_eq!(
        response.get("text").and_then(|t| t.as_str()),
        Some("This is extracted text from the document.")
    );
}
