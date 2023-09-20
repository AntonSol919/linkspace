.PHONY: all 

MD_PAGES := $(wildcard ./*.md)
ORG_PAGES := $(wildcard ./*.org)
PAGES := $(patsubst ./%.md,./%.html,$(MD_PAGES)) $(patsubst ./%.org,./%.html,$(ORG_PAGES)) 

pages: $(PAGES) 

%.html: %.org template.pml
	emacsclient --eval "(progn (switch-to-buffer (find-file-noselect \"./$<\")) (org-pandoc-export-to-html5))"

%.html: %.md template.pml
	pandoc -f markdown-native_divs -s ./$< --template ./template.pml  --metadata title=$@ -o $@
