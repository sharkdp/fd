# Architecture Overview of fd

fd is a simple, fast, and user-friendly alternative to the traditional `find` command. It is written in Rust and designed to make file searching intuitive and efficient.

## Key Features
- Intuitive syntax
- Smart case handling
- Integration with `.gitignore`
- Cross-platform support

## Project Structure
- **Main Module**: Handles the core logic for file searching and filtering.
- **Command Parsing**: Parses command-line arguments and configuration settings.
- **File Indexing**: Efficiently indexes and traverses the file system.
- **Filtering Engine**: Implements rules for file inclusion/exclusion (e.g., `.gitignore`).
- **Output Formatting**: Generates user-friendly output with color support.

## Purpose
fd aims to simplify common file search tasks while improving performance and usability over traditional tools like `find`.