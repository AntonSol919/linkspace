.PHONY: all 

pages: index.html basics.html lns.html about.html

%.html: %.md template.html
	/home/rs/Downloads/pandoc-3.1.3/bin/pandoc -f markdown-native_divs -s ./$< --template ./template.html --metadata title=$@ -o $@

