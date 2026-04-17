CC ?= gcc
CFLAGS ?= -O2 -Wall -Wextra
LDFLAGS ?=
DEBUG ?= 0

ifeq ($(DEBUG),1)
    CFLAGS += -g -DDEBUG
else
    CFLAGS += -s
endif

SRCDIR = src
OBJDIR = obj
BINDIR = bin

SOURCES = $(SRCDIR)/stratstop.c $(SRCDIR)/fb.c $(SRCDIR)/font.c $(SRCDIR)/logo.c
OBJECTS = $(patsubst $(SRCDIR)/%.c,$(OBJDIR)/%.o,$(SOURCES))
TARGET = $(BINDIR)/stratstop

.PHONY: all clean

all: $(TARGET)

$(TARGET): $(OBJECTS) | $(BINDIR)
	$(CC) $(LDFLAGS) -o $@ $^

$(OBJDIR)/%.o: $(SRCDIR)/%.c | $(OBJDIR)
	$(CC) $(CFLAGS) -c -o $@ $<

$(OBJDIR):
	mkdir -p $(OBJDIR)

$(BINDIR):
	mkdir -p $(BINDIR)

clean:
	rm -rf $(OBJDIR) $(BINDIR)
