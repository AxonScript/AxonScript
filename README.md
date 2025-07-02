# Build #0001 — Pre-Alpha Demo
## AxonScript

Hi, I'm Astral!

**AxonScript** is my own programming language.  
The original idea behind AxonScript was to create a language where you could easily build your own neural network.

At the moment, **native support for neural networks is not implemented yet**:

**Build #0001 — Pre-Alpha Demo**

But even now, **AxonScript is already Turing complete!**

---

## Tools I used to build AxonScript

- **Rust** — The entire language is written in Rust.
- **Logos** — A Rust library for fast and efficient lexical analysis.
- **llvm-sys** — A Rust binding for generating LLVM IR.
- **LLVM** — A powerful toolkit for building compilers.

---

## Compilation process

### 1. Lexical Analysis  
First, your source code goes through lexical analysis. All the words in your code are broken down into **tokens**.

### 2. Parsing  
After tokenization, the code is transformed into an **AST (Abstract Syntax Tree)**.

### 3. Semantic Analysis  
The AST is checked for semantic correctness and then optimized. In this stage, the AST is transformed into **HIR (High-level Intermediate Representation)**.

### 4. IR Generation  
From the HIR, the compiler generates **LLVM IR (Intermediate Representation)**.

### 5. Compilation  
LLVM IR is then optimized and compiled into machine code.

---

## Language Documentation

You can read the full documentation on our official website:  
👉 [https://axonscript.org/docs](https://axonscript.org/docs)

---

## Community

Join the community:  
🌐 [https://axonscript.org/community](https://axonscript.org/community)
