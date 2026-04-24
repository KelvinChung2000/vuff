// config: indent_width=4, indent_style=spaces
module m;
initial begin
x = 1;
end
endmodule
// expected -----
module m;
    initial begin
        x = 1;
    end
endmodule
