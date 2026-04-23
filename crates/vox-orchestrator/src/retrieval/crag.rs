//! Corrective RAG (CRAG) local heuristic router.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CragRoute {
    Retrieve,
    InContext,
    WebSearch,
}

pub struct CragRouter;

impl CragRouter {
    pub fn evaluate_query(q: &str) -> CragRoute {
        let text = q.to_lowercase();

        let web_patterns = ["news", "recent", "today", "latest", "breaking", "update"];
        for p in web_patterns {
            if text.contains(p) {
                return CragRoute::WebSearch;
            }
        }

        let local_patterns = [
            "my task",
            "this code",
            "earlier",
            "just said",
            "my file",
            "the current",
            "this function",
        ];
        for p in local_patterns {
            if text.contains(p) {
                return CragRoute::InContext;
            }
        }

        let factual_patterns = [
            "who", "what", "where", "history", "date", "how to", "why did",
        ];
        for p in factual_patterns {
            if text.contains(p) {
                return CragRoute::Retrieve;
            }
        }

        CragRoute::Retrieve
    }

    /// Evaluates the relevance of retrieved content against the original query using an LLM.
    pub async fn evaluate_document_relevance<F, Fut>(
        query: &str,
        document: &str,
        llm_fn: F,
    ) -> DocumentRelevance
    where
        F: Fn(&str, &str) -> Fut,
        Fut: std::future::Future<Output = Result<String, String>>,
    {
        let system = "You are a Document Relevance Evaluator for a Corrective RAG system.
Given a user query and a retrieved document, judge whether the document contains information relevant to answering the query.
Respond with EXACTLY ONE WORD:
- RELEVANT (if it contains useful information)
- IRRELEVANT (if it has nothing useful)
- AMBIGUOUS (if it is partially related but insufficient on its own)";

        // Truncate document to avoid blowing up context window for a lightweight evaluator
        let doc_preview = if document.len() > 8000 {
            &document[..8000]
        } else {
            document
        };
        let user = format!("QUERY: {}\n\nDOCUMENT:\n{}", query, doc_preview);

        match llm_fn(system, &user).await {
            Ok(resp) => {
                let resp_upper = resp.trim().to_uppercase();
                // Check exact matches or substrings safely
                if resp_upper == "RELEVANT"
                    || (resp_upper.contains("RELEVANT") && !resp_upper.contains("IRRELEVANT"))
                {
                    DocumentRelevance::Relevant
                } else if resp_upper.contains("IRRELEVANT") {
                    DocumentRelevance::Irrelevant
                } else {
                    DocumentRelevance::Ambiguous
                }
            }
            Err(_) => DocumentRelevance::Ambiguous, // Fallback to safe ambiguous state
        }
    }
}

/// The assessed relevance of a retrieved document
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentRelevance {
    Relevant,
    Irrelevant,
    Ambiguous,
}
