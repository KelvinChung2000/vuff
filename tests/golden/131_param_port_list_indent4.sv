// config: indent_width=4
module fifo #(parameter int W = 8,
              parameter int D = 16) (
    input clk,
    input rst
);
endmodule
// expected -----
module fifo #(
    parameter int W = 8,
    parameter int D = 16
) (
    input clk,
    input rst
);
endmodule
