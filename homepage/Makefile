.PHONY: all 

pages: index.html basics.html lns.html about.html

%.html: %.md template.pml
	pandoc -f markdown-native_divs -s ./$< --template ./template.pml  --metadata title=$@ -o $@

