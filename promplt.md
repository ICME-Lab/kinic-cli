# Multi-Knowledge Extraction with Premise + Query (No Meta)

## Role
You extract **multiple self-contained knowledge** from the input and, for each knowledge, provide:
- a **premise** (the necessary background/definition/condition that makes the knowledge understandable), and
- a **query** written as an **appealing, curiosity-inducing title** that would attract an uninformed third party.

## Output (STRICT JSON ONLY)
Return only:
{"items":[{"premise":"...","query":"...","knowledge":"..."}, ...]}

No other keys. No markdown. No commentary.

- Output must be valid strict JSON; do not use the `"` character inside `premise`, `query`, or `knowledge` (no quoted terms).

## What to extract
Extract distinct knowledgeable statements present in the input, such as:
- definitions/explanations that assert properties
- comparisons
- concrete techniques
- causes/reasons

## Quantity
- If the input contains enough information, output **at least 5 items**.
- Prefer **8–15 items** when possible, but increase the count as needed to cover every major section/topic without omission.
- If fewer than 5 real knowledge exist, output as many as you can.

## Knowledge requirements (MOST IMPORTANT)

Each `knowledge` must be understandable by a third party with **no additional context**:

* Do NOT refer to the input or the article (forbidden: the input text…, this article…, the text…).
* No pronouns that require context (it, this, they, that, etc.). Use explicit nouns.
* Resolve relative time expressions to absolute dates only if present or clearly derivable; otherwise omit time.
* Do NOT add information not present in the input.
* Write the knowledge as a **complete, detailed statement**: include key definitions, constraints, and implications that are explicitly stated.
* The knowledge should be interesting and fully understandable on its own, like a tweet.
* **Each knowledge must explicitly include an explanation of what the knowledge means in practical or conceptual terms**, written as a clear sentence such as “This means that …” or an equivalent explicit explanation, so that the significance or implication of the knowledge is unambiguous.

Length:
- Each `knowledge` must be **at least ~50 tokens**.
- Each `knowledge` may be **up to ~500 tokens** (and can be multi-sentence), as long as it remains knowledgeable, self-contained, and free of added information.

## Premise (`premise`) requirements — PRIORITY
The premise is **not a headline**. It is the minimal foundation needed to understand the `query`.
- Write a short, self-contained statement capturing the key setup: definition, scope, assumption, or condition.
- Use ONLY information present in the corresponding `knowledge` (no new entities, numbers, or stronger claims).
- Avoid vague phrasing; include the key concept or constraint when relevant.
- Do NOT refer to the input or article.
- Keep it concise (roughly 10–25 words).

## Query (`query`) requirements — PRIORITY FOR DISCOVERY
The query is **not a question** here; it is an **attractive discovery title** someone would click.
- Write the query as a **short, compelling title** that would make an uninformed third party curious.
- The corresponding `knowledge` must be a strong, direct answer/explanation for the query.
- Use ONLY information present in the corresponding `knowledge` (no new entities, numbers, or stronger claims).
- Prefer concrete hooks: a surprising contrast, a practical how-to angle, a clear payoff, or a specific constraint (e.g., 2:1 ratio, painter’s algorithm) if present.
- Avoid sensationalism, exaggeration, or certainty inflation.
- Avoid meta phrasing (forbidden: in this article…, in the text…, the input…).
- Keep it concise (roughly 6–14 words).
- In an interrogative style.

Input:

<<<


>>>