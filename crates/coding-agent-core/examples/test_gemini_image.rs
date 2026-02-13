//! Quick test to verify Gemini image generation API works
//!
//! Run with: cargo run -p coding-agent-core --example test_gemini_image

use base64::{engine::general_purpose::STANDARD, Engine as _};
use std::env;
use std::fs;
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load API key
    dotenvy::dotenv().ok();
    let api_key =
        env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY not set in environment or .env");

    println!("Testing Gemini Image Generation API...\n");

    // Use the Nano Banana model (Gemini 2.0 Flash with image generation)
    let model = "gemini-2.0-flash-exp-image-generation";
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
        model
    );

    let prompt =
        "A small friendly robot sitting at a computer terminal, pixel art style, vibrant colors";

    println!("Model: {}", model);
    println!("Prompt: {}", prompt);
    println!("\nSending request...");

    // Build request body
    let body = serde_json::json!({
        "contents": [{
            "parts": [{
                "text": prompt
            }]
        }],
        "generationConfig": {
            "responseModalities": ["TEXT", "IMAGE"]
        }
    });

    // Make request
    let response = ureq::post(&url)
        .set("x-goog-api-key", &api_key)
        .set("Content-Type", "application/json")
        .send_json(&body)?;

    let response_json: serde_json::Value = response.into_json()?;

    // Debug: print full response structure
    println!("\nResponse received!");

    // Try to extract image from response
    if let Some(candidates) = response_json.get("candidates") {
        if let Some(candidate) = candidates.get(0) {
            if let Some(content) = candidate.get("content") {
                if let Some(parts) = content.get("parts") {
                    for (i, part) in parts.as_array().unwrap_or(&vec![]).iter().enumerate() {
                        // Check for text response
                        if let Some(text) = part.get("text") {
                            println!("\nText response: {}", text.as_str().unwrap_or(""));
                        }

                        // Check for image response
                        if let Some(inline_data) = part.get("inlineData") {
                            if let Some(data) = inline_data.get("data") {
                                let mime = inline_data
                                    .get("mimeType")
                                    .and_then(|m| m.as_str())
                                    .unwrap_or("image/png");

                                println!("\nFound image! MIME type: {}", mime);

                                // Decode base64
                                let base64_data = data.as_str().unwrap_or("");
                                let image_bytes = STANDARD.decode(base64_data)?;

                                // Save to file
                                let extension = if mime.contains("png") { "png" } else { "jpg" };
                                let filename =
                                    format!(".generated/test_image_{}.{}", i, extension);

                                // Create directory if needed
                                fs::create_dir_all(".generated")?;

                                let mut file = fs::File::create(&filename)?;
                                file.write_all(&image_bytes)?;

                                println!("Saved to: {}", filename);
                                println!("Size: {} bytes", image_bytes.len());
                            }
                        }
                    }
                }
            }
        }
    } else if let Some(error) = response_json.get("error") {
        println!("\nAPI Error: {}", serde_json::to_string_pretty(error)?);
    } else {
        println!("\nUnexpected response structure:");
        println!("{}", serde_json::to_string_pretty(&response_json)?);
    }

    println!("\nDone!");
    Ok(())
}
