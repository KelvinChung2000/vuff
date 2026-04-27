// config: indent_width=4
module top;
    generate
        if (W == 8) begin
            assign q = d;
        end
    endgenerate
endmodule
// expected -----
module top;
    generate
        if (W == 8) begin
            assign q = d;
        end
    endgenerate
endmodule
