#!/usr/bin/env python3
"""
Simple syntax validation for Rust files
"""
import os
import re

def check_rust_file(filepath):
    """Basic syntax checks for Rust files"""
    with open(filepath, 'r') as f:
        content = f.read()
    
    issues = []
    
    # Check for basic syntax issues
    if content.count('{') != content.count('}'):
        issues.append("Mismatched braces")
    
    if content.count('(') != content.count(')'):
        issues.append("Mismatched parentheses")
    
    if content.count('[') != content.count(']'):
        issues.append("Mismatched brackets")
    
    # Check for common Rust patterns
    if 'use ' in content and not re.search(r'use\s+[\w:]+;', content):
        issues.append("Potential use statement syntax issue")
    
    return issues

def main():
    rust_files = [
        'src/main.rs',
        'src/cpu_metrics.rs',
        'src/shipping_service.rs'
    ]
    
    all_good = True
    for file in rust_files:
        if os.path.exists(file):
            issues = check_rust_file(file)
            if issues:
                print(f"Issues in {file}:")
                for issue in issues:
                    print(f"  - {issue}")
                all_good = False
            else:
                print(f"✓ {file} looks good")
        else:
            print(f"⚠ {file} not found")
    
    if all_good:
        print("\n✅ All files passed basic syntax validation")
    else:
        print("\n❌ Some issues found")

if __name__ == "__main__":
    main()
