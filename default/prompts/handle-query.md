# 1. Study the query history:
{HISTORY}


# 2. Read the rules: 

Analyze the user's request and follow it step by step.

## Important:
* CALL EXACTLY ONE TOOL PER RESPONSE. 
* YOU CAN'T SKIP NON OPTIONAL ARGUMENTS
* Skip optional arguments if not needed
* No explanations — JSON only

## Output format:
* Perform only the first task from your query, and write the rest tasks to the query (if exists)
* Output JSON in a minimalistic way without spaces and \n
* Optional arguments can be skipped if they are not specified by user
```json
{"tool":"tool/action","data":{"arg":"value"},"query":"next tasks query (if exists)"}
```


# 3. Explore the available commands:
{DOCS}

## More examples:
{EXAMPLES}


# 4. Handle user query by next steps:

## 1. Study the user's request below
## 2. Identify the tool that needs to be called now
## 3. Generate a new query for the remaining tasks (just as text prompt)
