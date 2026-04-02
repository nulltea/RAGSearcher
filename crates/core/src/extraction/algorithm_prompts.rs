/// Build the prompt for Pass 1: Algorithm Inventory — identify algorithms/protocols/schemes.
/// Paper text is injected via --append-system-prompt-file (not in this prompt).
pub fn algorithm_inventory_prompt() -> String {
    r#"You are identifying algorithms, protocols, and schemes in a research paper provided in the system prompt.

## Your Task
Find 1-5 distinct algorithms, protocols, schemes, or procedures described in the paper.
Only include items that have procedural/step-by-step content — skip vague mentions.

## What Counts as an Algorithm
- Named algorithms with defined steps (e.g., "Algorithm 1: FedAvg")
- Protocols with message flows or phases
- Schemes with defined construction/execution procedures
- Training/optimization procedures with specific update rules
- Data processing pipelines with defined stages

## What Does NOT Count
- General concepts mentioned without procedural detail
- Related work references to other papers' algorithms
- Evaluation metrics or benchmarks (unless they define a procedure)

## Output Format
Return ONLY a JSON object (no markdown fences):
{
    "paper_title": "Title of the paper",
    "algorithms": [
        {
            "id": "A1",
            "name": "Algorithm Name",
            "description": "1-2 sentence summary of what it does",
            "location": "Section name or number",
            "type": "algorithm|protocol|scheme|procedure"
        }
    ],
    "paper_type": "empirical|theoretical|system|survey"
}

## Guidelines
- Use sequential IDs: A1, A2, A3, etc.
- Keep the total response under 2000 tokens

The full paper text is available in the system prompt above."#
        .to_string()
}

/// Build the prompt for Pass 2: Algorithm Definition Extraction.
/// Paper text is injected via --append-system-prompt-file.
pub fn algorithm_extraction_prompt(inventory_json: &str) -> String {
    format!(
        r#"You are extracting algorithm definitions that a software engineer will use to implement them WITHOUT access to the original paper.

## CRITICAL REQUIREMENT
Each definition must be **self-contained and implementable**. The reader has NO access to the paper.
Every mathematical formula, threshold, hyperparameter, and decision rule must be explicit.

## Your Task
For each algorithm in the inventory below, produce a complete structured definition.
The full paper text is available in the system prompt — use it to extract precise details.

## Step Format Rules
- Each step is an **imperative action** ("Compute X", "Initialize Y", "For each Z, do W")
- Include implementation-level detail in the `details` field
- Use LaTeX for ALL math: `$...$` inline, `$$...$$` display

## LaTeX Conventions (token-efficient)
- Vectors: `$\mathbf{{x}}$` or `$x_i$`
- Sums: `$\sum_{{i=1}}^{{n}}$`
- Fractions: `$\frac{{a}}{{b}}$`
- Greek: `$\alpha, \beta, \theta, \nabla$`
- Sets: `$\mathcal{{D}}, \mathbb{{R}}^d$`
- Norms: `$\|x\|_2$`

## Output Format
Return ONLY a JSON object (no markdown fences):
{{
    "algorithms": [
        {{
            "rank": 1,
            "name": "Full Algorithm Name",
            "description": "2-3 sentence overview of purpose and approach",
            "steps": [
                {{
                    "number": 1,
                    "action": "Initialize model parameters",
                    "details": "Set $\\theta_0 \\sim \\mathcal{{N}}(0, 0.01)$ for all layers. Initialize learning rate $\\eta = 0.01$.",
                    "math": "$\\theta_0 \\in \\mathbb{{R}}^d$"
                }}
            ],
            "inputs": [
                {{
                    "name": "training_data",
                    "type": "Dataset of (x, y) pairs",
                    "description": "Labeled samples where $x \\in \\mathbb{{R}}^d$, $y \\in \\{{0,1\\}}$"
                }}
            ],
            "outputs": [
                {{
                    "name": "trained_model",
                    "type": "Model parameters $\\theta^*$",
                    "description": "Optimized parameters after convergence"
                }}
            ],
            "preconditions": [
                "Data is IID sampled from distribution $\\mathcal{{D}}$",
                "Loss function $\\ell$ is differentiable"
            ],
            "complexity": "O(T \\cdot n \\cdot d) where T=rounds, n=samples, d=dimensions",
            "mathematical_notation": "$$\\theta_{{t+1}} = \\theta_t - \\eta \\nabla \\ell(\\theta_t; x, y)$$",
            "pseudocode": "function train(data, T, eta):\n  theta = init_params()\n  for t in 1..T:\n    for (x, y) in data:\n      grad = compute_gradient(theta, x, y)\n      theta = theta - eta * grad\n  return theta",
            "tags": ["optimization", "gradient-descent"],
            "confidence": "high"
        }}
    ]
}}

## Self-Containment Check
Before finalizing each algorithm, verify:
1. Could someone implement this from ONLY your definition?
2. Are all variables defined before use?
3. Are all hyperparameters/thresholds specified with values or ranges?
4. Are termination conditions explicit?
5. Are edge cases mentioned in preconditions?

If any answer is "no", add the missing information.

## Guidelines
- Extract 1-5 algorithms, ranked by importance
- Keep step count between 3-15 per algorithm
- Pseudocode should be language-agnostic (no specific syntax)
- Keep the total response under 6000 tokens

## Algorithm Inventory
{inventory_json}"#
    )
}

/// Build the prompt for Pass 3: Algorithm Verification.
pub fn algorithm_verification_prompt(algorithms_json: &str) -> String {
    format!(
        r#"You are verifying the quality and implementability of extracted algorithm definitions.

## Your Task
Review each algorithm definition for completeness, correctness, and self-containment.

## Checks to Perform

### 1. Implementability
- Can each algorithm be implemented from the definition alone (no paper access)?
- Are all variables defined before use?
- Are termination conditions explicit?
- Are hyperparameters specified with concrete values or ranges?

### 2. Step Completeness
- Are steps ordered correctly?
- Are there missing intermediate steps?
- Is mathematical notation consistent across steps?

### 3. Mathematical Correctness
- Is LaTeX well-formed?
- Are dimensions/types consistent?

## Output Format
Return ONLY a JSON object (no markdown fences):
{{
    "verification_status": "pass|warn|fail",
    "completeness_issues": [
        {{
            "algorithm_rank": 1,
            "issue": "Step 3 references $\\alpha$ but it is not defined in inputs or earlier steps",
            "severity": "error|warning"
        }}
    ],
    "citation_issues": [],
    "overall_quality": "high|medium|low",
    "improvement_suggestions": ["Add learning rate schedule to Algorithm 1"]
}}

## Verification Status Guidelines
- pass: All algorithms are implementable, no errors
- warn: Minor issues (warnings only) that don't block implementation
- fail: Critical issues — undefined variables, missing steps, or invalid citations

## Guidelines
- Be concise. Keep the total response under 2000 tokens.

## Extracted Algorithms
{algorithms_json}"#
    )
}
