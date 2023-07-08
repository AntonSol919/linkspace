.PHONY: all 

pages: index.html why.html lns.html about.html code.html

%.html: %.org template.pml
	emacsclient --eval "(progn (switch-to-buffer (find-file-noselect \"./$<\")) (org-pandoc-export-to-html5))"

%.html: %.md template.pml
	pandoc -f markdown-native_divs -s ./$< --template ./template.pml  --metadata title=$@ -o $@
