/**
 * SSOT Explainer Registry for SCIENTIA Research Pipeline Stages.
 *
 * Maps canonical research stages to plain English summaries, why they matter,
 * mathematical/algorithmic underpinnings, and user/debugger actions.
 */

export interface StageExplainer {
  id?: string;
  plainTitle: string;
  plainSummary: string;
  whyItMatters: string;
  mathOrAlgorithm: string;
  action: string;
  howItWorks?: string;
  technicalMath?: {
    formulaLatex: string;
    parameters?: Record<string, string | number>;
    description: string;
  };
  actions?: Array<{
    id: string;
    label: string;
    tooltip: string;
    role: 'step' | 'audit' | 'inspect';
  }>;
}

export const RESEARCH_STAGE_EXPLAINERS: Record<string, StageExplainer> = {
  queued: {
    id: 'queued',
    plainTitle: 'Queued for Research',
    plainSummary: 'Waiting in the orchestrator pipeline for worker allocation and execution.',
    whyItMatters: 'Ensures runs execute in order without overloading background daemon capacity.',
    mathOrAlgorithm: 'FIFO Queue Ordering: Priority = Timestamp + PriorityWeight',
    action: 'Wait for worker allocation',
    howItWorks: 'Enqueues research task into the persistent orchestrator queue and monitors daemon availability.',
    technicalMath: {
      formulaLatex: 'P(T) = t_{submit} - w_{priority} \\cdot \\Delta t',
      parameters: { defaultPriority: 1.0 },
      description: 'Dynamic priority queue ordering function for scheduled daemon jobs.'
    },
    actions: [
      { id: 'view-queue', label: 'View Queue', tooltip: 'Inspect current orchestrator hopper queue', role: 'inspect' }
    ]
  },
  planning: {
    id: 'planning',
    plainTitle: 'Analyzing & Planning Sub-Topics',
    plainSummary: 'Splitting your question into focused sub-queries to ensure complete coverage.',
    whyItMatters: 'Prevents shallow answers by examining different perspectives and tradeoffs independently.',
    mathOrAlgorithm: '\\mathcal{H}(Q) = -\\sum_{i=1}^n P(w_i) \\log_2 P(w_i)',
    action: 'Dispatch sub-queries',
    howItWorks: 'Measures topic breadth (information entropy) and picks discriminating technical keywords.',
    technicalMath: {
      formulaLatex: '\\mathcal{H}(Q) = -\\sum_{i=1}^n P(w_i) \\log_2 P(w_i)',
      parameters: { threshold: '2.5 bits', maxSubqueries: 8 },
      description: 'Query Information Entropy determines multi-query fan-out breadth.'
    },
    actions: [
      { id: 'step-next', label: 'Step to Retrieval', tooltip: 'Dispatch search queries across enabled engines', role: 'step' }
    ]
  },
  retrieving: {
    id: 'retrieving',
    plainTitle: 'Retrieving Sources across Multiple Lanes',
    plainSummary: 'Querying academic, encyclopedic, and web search engines simultaneously.',
    whyItMatters: 'Cross-domain retrieval prevents single-source bias and captures empirical evidence.',
    mathOrAlgorithm: 'RRF(d) = \\sum_{m \\in M} \\frac{1}{k + r_m(d)}',
    action: 'Fuse results across engines',
    howItWorks: 'Executes parallel searches across Wikipedia, OpenAlex, arXiv, Tavily, and SearXNG, merging with Reciprocal Rank Fusion.',
    technicalMath: {
      formulaLatex: 'RRF(d) = \\sum_{m \\in M} \\frac{1}{k + r_m(d)}',
      parameters: { k: 60 },
      description: 'Reciprocal Rank Fusion merges ranked result lists without needing cross-engine score calibration.'
    },
    actions: [
      { id: 'inspect-sources', label: 'Inspect Sources', tooltip: 'View raw hits collected from each engine', role: 'inspect' }
    ]
  },
  verifying_claims: {
    id: 'verifying_claims',
    plainTitle: 'Verifying Claims & Neutrality',
    plainSummary: 'Testing factual statements against source passages and checking for conflicting evidence.',
    whyItMatters: 'Catches hallucinations, outdated data, and misinformation before synthesis.',
    mathOrAlgorithm: 'S_{verify}(c) = \\max_{p \\in P} \\cos(e_c, e_p) \\cdot (1 - P_{contradiction})',
    action: 'Filter uncorroborated claims',
    howItWorks: 'Compares extracted claim embeddings against retrieved passage embeddings and applies NLI contradiction penalties.',
    technicalMath: {
      formulaLatex: 'S_{verify}(c) = \\max_{p \\in P} \\cos(e_c, e_p) \\cdot (1 - P_{contradiction})',
      parameters: { tau_threshold: 0.72 },
      description: 'Natural Language Inference verification score for extracted claims.'
    },
    actions: [
      { id: 'audit-claims', label: 'Audit Claims', tooltip: 'Review verified vs rejected claim statements', role: 'audit' }
    ]
  },
  synthesizing: {
    id: 'synthesizing',
    plainTitle: 'Synthesizing Evidence & Narrative',
    plainSummary: 'Organizing verified findings into a structured technical summary with citations.',
    whyItMatters: 'Produces actionable, readable answers cited with verifiable sources.',
    mathOrAlgorithm: 'DAG Topological Sort + Citation Grounding: G = (V_{claims}, E_{inference})',
    action: 'Compose synthesis draft',
    howItWorks: 'Constructs an inference graph connecting evidence to conclusions and streams synthesis markdown.',
    technicalMath: {
      formulaLatex: 'G = (V_{claims}, E_{inference})',
      parameters: { minCoverage: 0.85 },
      description: 'Directed Acyclic Evidence Graph ensuring logical ordering of verified claims.'
    },
    actions: [
      { id: 'inspect-draft', label: 'Inspect Draft', tooltip: 'Preview the synthesis draft before publishing', role: 'inspect' }
    ]
  },
  auditing_citations: {
    id: 'auditing_citations',
    plainTitle: 'Auditing Citations & Attribution',
    plainSummary: 'Verifying that every factual claim links directly to a verifiable URL or paper DOI.',
    whyItMatters: 'Guarantees academic rigor and zero broken or fabricated references.',
    mathOrAlgorithm: 'Precision = \\frac{|Verified Citations|}{|Total Citations|} \\ge \\tau_{threshold}',
    action: 'Validate source URLs and DOIs',
    howItWorks: 'Checks all citations for URL reachability, canonical DOI formatting, and domain reputation.',
    technicalMath: {
      formulaLatex: 'Precision_{cite} = \\frac{\\sum_{i=1}^N \\mathbb{I}(\\text{valid}(c_i))}{N}',
      parameters: { minPrecision: 0.95 },
      description: 'Citation precision check over all inline references.'
    },
    actions: [
      { id: 'audit-links', label: 'Audit Links', tooltip: 'Verify reachability and DOI status', role: 'audit' }
    ]
  },
  persisting: {
    id: 'persisting',
    plainTitle: 'Persisting Synthesis & Claims',
    plainSummary: 'Writing research documentation, frontmatter, and claim records to the local database and filesystem.',
    whyItMatters: 'Enables long-term recall, offline access, and cross-session knowledge building.',
    mathOrAlgorithm: 'Atomic SQLite Transaction + SHA-256 Content Invariant Verification',
    action: 'Commit records to VoxDb and docs tree',
    howItWorks: 'Writes markdown documents with Astro frontmatter and updates VoxDb claim tables inside a database transaction.',
    technicalMath: {
      formulaLatex: 'H_{doc} = \\text{SHA-256}(\\text{content} \\parallel \\text{frontmatter})',
      parameters: { isolation: 'IMMEDIATE' },
      description: 'Content integrity hash ensuring reproducible storage.'
    },
    actions: [
      { id: 'view-files', label: 'View Files', tooltip: 'Open generated research files in filesystem', role: 'inspect' }
    ]
  },
  completed: {
    id: 'completed',
    plainTitle: 'Research Complete',
    plainSummary: 'All research phases finished successfully with verified claims and published findings.',
    whyItMatters: 'Provides high-confidence technical answers ready for engineering and decision making.',
    mathOrAlgorithm: 'Pipeline Status Terminal State: Success (Exit Code 0)',
    action: 'View research document',
    howItWorks: 'Signals session completion to the GUI and presents final synthesized results.',
    technicalMath: {
      formulaLatex: '\\text{State} = \\text{TERMINAL\\_SUCCESS}',
      parameters: { exitCode: 0 },
      description: 'Terminal pipeline completion state.'
    },
    actions: [
      { id: 'open-doc', label: 'Open Document', tooltip: 'Read synthesized research report', role: 'step' }
    ]
  }
};
