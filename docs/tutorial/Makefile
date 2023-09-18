.PHONY: all 

MD_PAGES := $(wildcard ./*.md)
ORG_PAGES := $(wildcard ./*.org)
PAGES := $(patsubst ./%.md,./%.html,$(MD_PAGES)) $(patsubst ./%.org,./%.html,$(ORG_PAGES)) 

all : $(PAGES) 

%.html: %.org template.pml
	emacsclient --eval "(progn (switch-to-buffer (find-file-noselect \"./$<\")) (org-pandoc-export-to-html5))"

%.html: %.md
	pandoc --embed-resources --standalone $< > $@

