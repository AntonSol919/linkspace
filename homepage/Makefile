.PHONY: all 

pages: index.html basics.html basics.old.html why.html lns.html about.html

%.html: %.md template.html
	pandoc -s ./$< --template ./template.html --metadata title=$@ -o $@
