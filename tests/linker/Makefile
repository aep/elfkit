SUBDIRS=$(shell find * -maxdepth 1 -type d)

.PHONY: test
test:
	for dir in $(SUBDIRS); do \
		$(MAKE) -C $$dir test; \
	done

.PHONY: clean
clean:
	for dir in $(SUBDIRS); do \
		$(MAKE) -C $$dir clean; \
	done


