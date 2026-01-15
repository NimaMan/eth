# The ETH Documentation Principle

**Core Directive:** Documentation must be **Self-Replicable**.

Every `README.md` in this repository is not just an introduction; it is a standalone chapter of the System Manual. A developer should be able to read the `README` at any level of the directory tree and have enough information to theoretically reconstruct the code within that directory from scratch.

## The Hierarchy of Knowledge

1.  **Root Level (README.md): The Architect's Blueprint**
    *   Defines the **Vision**: Why does this project exist? (e.g., High-performance Ethereum simulation via Reth Node).
    *   Defines the **Architecture**: What are the major components (SDK vs. Tools)? How do they interact?
    *   *Goal:* Allows a reader to scaffold the entire workspace structure and understand the data flow.

2.  **Folder Level (`<folder_path>/README.md`): The Engineer's Spec**
    *   Every meaningful folder within the project should contain a `README.md` file.
    *   This includes `src/`, sub-folders within `src/`, and any other significant directories.
    *   Defines the **Problem Domain**: What specific technical challenge does this folder/crate solve?
    *   Defines the **Logic**: How does the solution work?
    *   *Goal:* Allows a reader to understand the purpose and high-level design of the components within that folder.

3.  **File Level (Source Code `//!` in `.rs` files): The Coder's Guide**
    *   **Format:** Use Rust standard module-level documentation comments (`//!`) at the very top of `.rs` files.
    *   **Content:** Defines the **Implementation Details**: Specific patterns, tricky dependencies, and helper functions.
    *   *Goal:* Allows a reader to write the specific lines of code, handling edge cases and library quirks, without context switching between code and a separate markdown file.


## Rule of Thumb

**"If I deleted the code but kept the README, could a skilled engineer rewrite it?"**
If the answer is No, the documentation is incomplete.
