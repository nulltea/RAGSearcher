/// Build the prompt for Pass 1: Evidence Inventory extraction.
pub fn evidence_inventory_prompt(text: &str) -> String {
    format!(
        r#"You are extracting concrete evidence from a research paper.

## Your Task
Extract 12-25 distinct pieces of evidence from the paper. Each piece must be a direct quote or specific finding that can be cited later.

## What Counts as Evidence
- Direct quotes from the paper (use exact wording)
- Quantitative results (numbers, percentages, measurements)
- Experimental findings with specific outcomes
- Defined terms or concepts with their definitions
- Methodology descriptions with specific details
- Limitations or failure modes explicitly mentioned
- Comparisons between approaches with specific metrics

## What Does NOT Count as Evidence
- Your interpretations or inferences
- General statements without specifics
- Background information not central to the paper's contribution
- Vague claims without supporting details

## Evidence Types
- finding: A research result or discovery
- definition: A term or concept being defined
- mechanism: How something works
- limitation: A weakness or boundary condition
- comparison: Comparison between methods/approaches
- claim: An assertion made by the authors
- methodology: A specific method or technique description
- result: A quantitative or qualitative outcome

## Output Format
Return ONLY a JSON object (no markdown fences):
{{
    "paper_title": "Title of the paper",
    "evidence_items": [
        {{
            "id": "E1",
            "quote": "exact quote from paper",
            "location": "section name",
            "type": "finding|definition|mechanism|limitation|comparison|claim|methodology|result",
            "importance": "high|medium|low"
        }}
    ],
    "paper_type": "empirical|theoretical|system|survey|position",
    "core_contribution_summary": "1-2 sentence summary"
}}

## Guidelines
- Use sequential IDs: E1, E2, E3, etc.
- Quote exactly - do not paraphrase
- Mark importance as "high" for core findings, "medium" for supporting evidence, "low" for minor details
- Aim for 12-25 evidence items

## Paper Text
{text}"#
    )
}

/// Build the prompt for Pass 2: Pattern Extraction with evidence citations.
pub fn pattern_extraction_prompt(text: &str, evidence_json: &str) -> String {
    format!(
        r#"You are extracting research patterns grounded in evidence.

## Your Task
Extract 3-8 patterns from the research paper. Each pattern must cite evidence from the provided evidence inventory using [E#] format.

## Claim/Evidence/Context Framework

### Claim
The main insight, assertion, or finding. MUST cite evidence using [E#] format.

### Evidence
Supporting quotes, data, examples from the source. MUST cite evidence using [E#] format.

### Context
Implications, limitations, connections to other ideas. MUST cite evidence using [E#] format.

## CRITICAL RULES
1. Every assertion MUST cite evidence using [E#] format (e.g., [E1], [E3, E5])
2. Claim/Evidence/Context must describe DIFFERENT aspects - never repeat the same content
3. No claim without evidence - if no evidence supports a claim, use null for that field
4. Extract 3-8 patterns per paper, ranked by importance
5. Each pattern needs a short name (3-6 words)
6. Use only evidence IDs from the provided inventory

## Available Tags
- methodology, finding, limitation, future-work, experimental-setup, theoretical, practical

## Output Format
Return ONLY a JSON object (no markdown fences):
{{
    "patterns": [
        {{
            "rank": 1,
            "name": "Pattern Name (3-6 words)",
            "claim": "Main assertion citing evidence [E1, E3]",
            "evidence": "Supporting data citing evidence [E2, E5]",
            "context": "Why this matters citing evidence [E4]",
            "tags": ["methodology", "finding"],
            "evidence_ids": ["E1", "E2", "E3", "E4", "E5"],
            "confidence": "high|medium|low"
        }}
    ],
    "total_evidence_used": 12,
    "gaps_identified": ["No evidence for claim X"]
}}

## Evidence Inventory
{evidence_json}

## Paper Text
{text}"#
    )
}

/// Build the prompt for Pass 3: Verification of extracted patterns.
pub fn verification_prompt(evidence_json: &str, patterns_json: &str) -> String {
    format!(
        r#"You are verifying the quality of pattern extraction from a research paper.

## Your Task
Review the extracted patterns against the evidence inventory and identify any issues.

## Checks to Perform

### 1. Citation Validity
- Does each [E#] reference exist in the evidence inventory?
- Flag any citations to non-existent evidence as errors.

### 2. Citation Accuracy
- Does the claim/evidence/context accurately represent what the cited evidence says?
- Flag any misrepresentations or overstatements.

### 3. Evidence Coverage
- Which evidence items from the inventory were NOT used?
- High-importance evidence that isn't cited may indicate gaps.

### 4. Component Separation
- Are claim/evidence/context truly different content?
- Flag if the same content is repeated across fields.

## Output Format
Return ONLY a JSON object (no markdown fences):
{{
    "verification_status": "pass|warn|fail",
    "citation_issues": [
        {{
            "pattern_rank": 1,
            "field": "claim|evidence|context",
            "issue": "description",
            "severity": "error|warning"
        }}
    ],
    "unused_evidence": ["E8", "E12"],
    "accuracy_concerns": [
        {{
            "pattern_rank": 1,
            "evidence_id": "E3",
            "concern": "Pattern claims X but evidence only shows Y"
        }}
    ],
    "overall_quality": "high|medium|low",
    "improvement_suggestions": ["Consider citing E8 for additional support"]
}}

## Verification Status Guidelines
- pass: No errors, few or no warnings, good evidence coverage
- warn: No errors but has warnings OR significant unused evidence
- fail: Has errors (invalid citations) OR major accuracy concerns

## Evidence Inventory
{evidence_json}

## Extracted Patterns
{patterns_json}"#
    )
}
