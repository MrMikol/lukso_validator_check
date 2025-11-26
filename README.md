🟣 LUKSO Validator Checks
================================

This application crawls the paginated **Included Deposits** table on the LUKSO Dora Explorer website and detects validators whose **Validator State** contains:

-   🟥 **Red ("danger") power icon**

-   🟨 **Yellow ("warning") power icon**

-   (optional) 🟩 **Green ("success") power icon**

It records:

-   **Page number** where the validator was found

-   **Full URL** of that page

-   **Validator index**

-   **Validator public key**

-   **Health color** (red / yellow / green)