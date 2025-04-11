#!/usr/bin/env fish

set DATES "2025-01" "2025-02" "2025-03"

for date in $DATES
    cargo run -- $date 2>/dev/null > $date.txt
end

for date in $DATES
    pandoc $date.txt -o KUP-$date.pdf --pdf-engine=xelatex
    rm $date.txt
end
