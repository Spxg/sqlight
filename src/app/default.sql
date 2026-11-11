-- Welcome to SQLight! Everything runs locally in your browser.
-- Click Run, or press Ctrl+Enter / Cmd+Enter, to try this bookstore demo.

CREATE TABLE IF NOT EXISTS demo_books (
    id INTEGER PRIMARY KEY,
    title TEXT NOT NULL,
    price_cents INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS demo_sales (
    id INTEGER PRIMARY KEY,
    book_id INTEGER NOT NULL,
    quantity INTEGER NOT NULL
);

-- Fixed IDs make this example safe to run again with Keep Context enabled.
INSERT INTO demo_books (id, title, price_cents) VALUES
    (1, 'The Rust Book', 3500),
    (2, 'SQLite in Practice', 2800),
    (3, 'SQL for Curious Minds', 2400),
    (4, 'Small, Fast, Reliable', 3200)
ON CONFLICT (id) DO UPDATE SET
    title = excluded.title,
    price_cents = excluded.price_cents;

INSERT INTO demo_sales (id, book_id, quantity) VALUES
    (1, 1, 2),
    (2, 2, 3),
    (3, 1, 1),
    (4, 3, 4)
ON CONFLICT (id) DO UPDATE SET
    book_id = excluded.book_id,
    quantity = excluded.quantity;

-- Browse the catalog. Prices are stored as cents and displayed as decimals.
SELECT title, price_cents / 100.0 AS price
FROM demo_books
ORDER BY price_cents, title;

-- Best sellers, including the book with no sales yet.
SELECT
    b.title,
    COALESCE(SUM(s.quantity), 0) AS copies_sold,
    ROUND(COALESCE(SUM(s.quantity * b.price_cents), 0) / 100.0, 2) AS revenue
FROM demo_books AS b
LEFT JOIN demo_sales AS s ON s.book_id = b.id
GROUP BY b.id, b.title, b.price_cents
ORDER BY copies_sold DESC, revenue DESC, b.title;
