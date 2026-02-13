## Generate colorschemes based on images

Currently outputs 16 (if possible) distinct colors from an image.

Output methods:
- Area average
- K-Means
- 16 ANSI (normal: 1-8, bright 9-16)

## Templates

Templates are placed in ~/.config/pal/

Generated templates are placed in ~/.cache/pal/

Syntax:

Variables must start with @ and be surrounded with backticks: 
- ``@background``
- ``@foreground`` 
- ``@color<1-16>``

Example templates are provided in examples folder.
