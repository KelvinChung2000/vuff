module top #(parameter int WIDTH = 8, parameter int DEPTH = 16,
             parameter int FLAGS = 4) (
  input clk,
  input rst
);
endmodule
// expected -----
module top #(
  parameter int WIDTH = 8,
  parameter int DEPTH = 16,
  parameter int FLAGS = 4
) (
  input clk,
  input rst
);
endmodule
