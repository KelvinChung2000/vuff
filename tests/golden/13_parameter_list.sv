module m #(parameter int WIDTH = 8,
           parameter int DEPTH = 16) (input wire clk
);
localparam int MAX = WIDTH * DEPTH;
endmodule
// expected -----
module m #(
  parameter int WIDTH = 8,
  parameter int DEPTH = 16
) (
  input wire clk
);
  localparam int MAX = WIDTH * DEPTH;
endmodule
