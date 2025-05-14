Summary of vendored changes:

* Void nodes did not auto-close, we insert a closing tag for them.
* The `pre` and `textarea` tag did not respect whitespace, added flag to cease
  inserting whitespace until the end of the tag;
