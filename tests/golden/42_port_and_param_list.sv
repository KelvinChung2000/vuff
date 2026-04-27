module m #(parameter int W=8)(input [W+1:0] a);
endmodule
// expected -----
module m #(parameter int W = 8) (input [W + 1:0] a);
endmodule
