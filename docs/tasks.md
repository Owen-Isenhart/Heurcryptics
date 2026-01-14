# Current To Do
## Data & Ingestion (The Foundation)

- Synthetic Data Generator: Automatically applies ciphers (Caesar, XOR, Base64) to "Clean" files to create perfectly labeled training sets.
- Magic-Byte Identification: Uses the infer crate to identify the true type of "Clean" files during ingestion, preventing extension-spoofing errors.
- SHA-256 Deduplication: Hashes every input file. If you try to train on cat.jpg twice, the tool skips it to prevent model bias.

## Statistical Heuristics (The Current Engine)

- Rolling Entropy (Heatmap): Scans files in small blocks to find where data types change (e.g., finding a hidden ZIP at the end of a JPG).
- Chi-Square Distribution: An advanced test to mathematically distinguish between "Highly Compressed Media" and "Truly Random Encryption."
- Shift-Cipher Guessing: Automatically tries all 256 Caesar shifts on unknown data to see if any reveal a known "Prose" or "Code" signature.

## Machine Learning (The CNN Layer)
- Composite Labeling: Training the CNN on categories like jpg_caesar so it learns to see through the cipher.
- Standardized Tiling: A utility that "chops" large files into uniform 256×256 Hilbert maps so the CNN always sees the same size.
- Model Confidence Reporting: Tells you why it made a guess (e.g., "70% sure it's a JPG because of the top-left corner texture").

## UI & Visual Tools (The Dashboard)
- Interactive Hilbert Web-View: A Next.js dashboard where you can hover over a pixel to see the hex value and its position in the original file.
- Visual Diffing: A tool to subtract one Hilbert Map from another to see exactly how a cipher "scrambles" the data.