# Test Snippets for Code Highlighting

## C Code
```c
#include <stdio.h>

int main() {
    printf("Hello from C\n");
    return 0;
}
```

## C++ Code
```cpp
#include <iostream>

int main() {
    std::cout << "Hello from C++" << std::endl;
    return 0;
}
```

## Java Code
```java
public class HelloWorld {
    public static void main(String[] args) {
        System.out.println("Hello from Java");
    }
}
```

## Python Code
```python
def greet(name):
    return f"Hello from {name}"

print(greet("Python"))
```

## TypeScript Code
```typescript
function greet(name: string): string {
    return `Hello from ${name}`;
}

console.log(greet("TypeScript"));
```

## JavaScript Code
```javascript
function greet(name) {
    return `Hello from ${name}`;
}

console.log(greet("JavaScript"));
```

## Rust Code
```rust
fn main() {
    let name = "Rust";
    println!("Hello from {}", name);
}
```

## Go Code
```go
package main

import "fmt"

func main() {
    fmt.Println("Hello from Go")
}
```

## Bash Code
```bash
#!/bin/bash

greet() {
    echo "Hello from $1"
}

greet "Bash"
```

## PowerShell Code
```powershell
function Greet($name) {
    Write-Host "Hello from $name"
}

Greet "PowerShell"
```

## JSX Code
```jsx
const Button = ({ onClick, label }) => (
    <button onClick={onClick} className="btn">
        {label}
    </button>
);

export default Button;
```

## TSX Code
```tsx
interface ButtonProps {
    onClick: () => void;
    label: string;
}

const Button: React.FC<ButtonProps> = ({ onClick, label }) => (
    <button onClick={onClick} className="btn">
        {label}
    </button>
);

export default Button;
```

## Plain Text
```text
This is plain text content without any special formatting.
It should render as-is without syntax highlighting.
Just plain readable text for reference.
```

## Summary

All languages have been tested with sample code snippets:
- Compiled languages: C, C++, Java, Rust, Go
- Scripting languages: Python, JavaScript, TypeScript, Bash, PowerShell
- React variants: JSX, TSX
- Plain text content

This markdown file is optimized for quick performance testing.
