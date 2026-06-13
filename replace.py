import re
with open('src/index.html', 'r', encoding='utf-8') as f:
    content = f.read()
new_content = re.sub(r'<script>.*?</script>', '<script src="main.js"></script>', content, flags=re.DOTALL)
with open('src/index.html', 'w', encoding='utf-8') as f:
    f.write(new_content)
print("Done")
