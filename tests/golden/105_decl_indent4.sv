// config: indent_width=4
module m;
    wire   [7:0]   bus;
    logic  flag;
    reg    [3:0]   nib  = 4'h0;
endmodule
// expected -----
module m;
    wire [7:0] bus;
    logic flag;
    reg [3:0] nib = 4'h0;
endmodule
