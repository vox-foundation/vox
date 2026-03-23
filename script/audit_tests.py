import os
import glob

print("Crate | Src | Tests")
print("---|---|---")

for d in os.listdir("crates"):
    path = os.path.join("crates", d)
    if os.path.isdir(path):
        src_files = glob.glob(f"{path}/src/**/*.rs", recursive=True)
        test_files = glob.glob(f"{path}/tests/**/*.rs", recursive=True)
        
        # Also let's check for inline tests in src files
        inline_test_count = 0
        for f in src_files:
            try:
                with open(f, 'r', encoding='utf-8') as file:
                    content = file.read()
                    if "#[test]" in content or "#[tokio::test]" in content:
                        inline_test_count += 1
            except:
                pass
                
        print(f"{d} | {len(src_files)} | {len(test_files)} (Inline src with tests: {inline_test_count})")
