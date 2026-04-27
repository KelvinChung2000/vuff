// config: indent_width=4
package pkg;
    typedef logic [7:0] byte_t;
    parameter int W = 8;
endpackage
// expected -----
package pkg;
    typedef logic [7:0] byte_t;
    parameter int W = 8;
endpackage
