.PHONY: all 

pages: index.html basics.html lns.html about.html code.html

%.md: %.org
	emacsclient --eval "(progn (switch-to-buffer (find-file-noselect \"./$<\")) (org-md-export-to-markdown))"

%.html: %.md template.pml
	pandoc -f markdown-native_divs -s ./$< --template ./template.pml  --metadata title=$@ -o $@

