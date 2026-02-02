When we have failing tests, it's important to start from the bottom up

1. Write the failing tests for the scenario
2. Work from the bottom up:
- Does the data flow to the parser?
- Does the data flow to the AST?
- Does the data flow to the extraction?
- Does the data flow to the resolver?
3. Add logging if you need to.
4. What is the expected behaviour vs the anticipated behaviour?
5. What is the best solution for this fix? Where does the chain of responsibility sit?
6. Implement the fix.