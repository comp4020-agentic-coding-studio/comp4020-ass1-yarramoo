# Assignment 1 reflection

**The breakthrough that moved the work forward** 
Something that required intervention was Claude rapidly developing and not tracking
work with commits that map nicely to features being developed. I had to ask Claude to 
go back, selectively add files (it did some more clever git magic to ensure testing in-between 
commits still passed), and commit them into logically seperate blocks. Once this was done, 
a note was added to `CLAUDE.md` to keep up with adding small, understandable commits. 

Also notable, development seemed quite easy for Claude-code due to the tech stack. 
Using an existing ray-tracing Rust library with simple abstractions led to 
zero friction and issues when adding features to the ray-tracer itself. Claude 
quickly got in the habit of adding unit tests inline using Rust's testing framework. 

**What this changed about who I want to be as a developer** Firstly git hygeine will be a 
standard addition to my `CLAUDE.md` files from now on. 

Secondly it is more clear to me that having a better idea of what tools AI finds cheaper and 
easier to work with is valuable. How long do tests take to run? With what precision can they be run 
automatically? How many tokens are used to refactor in one tech stack vs another? 